// ext/thapthim/src/spell.rs
//
// Word-level lexical spelling correction — the Norvig / noisy-channel model:
//   1. generate dictionary words within a bounded Damerau-Levenshtein distance of the input,
//   2. rank them by  log10(unigram frequency) - LAMBDA * edit_distance,
//   3. return the ranked list (`suggest`) or the top pick (`correct`).
//
// Candidate generation is a brute-force bounded edit-distance scan over the ~61k-word dictionary
// with a character-length prefilter. In Rust that is sub-millisecond per query, so no trie/automaton
// is needed at this stage (a trie would matter for the sentence-level `correct_sent` lattice later).
//
// The dictionary is PyThaiNLP's cleaned `thai_words` (assets/spell_words.txt); frequencies come from
// the same Kneser-Ney unigram table the segmenter ships (assets/kn_words_unigrams.txt). Both are
// std_normalized, and every query is normalized the same way, so the lexical layer sits cleanly on
// top of the orthographic one (std_normalize).

use rustc_hash::FxHashSet;

pub const MAX_EDITS: usize = 2;
pub const SUGGEST_TOP_N: usize = 10;
const LAMBDA: f64 = 2.0; // edit-vs-frequency balance; validated on VISTEC/synthetic typos
// Default edit-vs-bigram balance for correct_sentence (the LM term is a natural-log KN bigram, so a
// larger λ than the log10 isolated case). Overridable per call for tuning; see the FFI in lib.rs.
pub const DEFAULT_SENT_LAMBDA: f64 = 3.0;

// A Thai letter/vowel/tone/sign (U+0E01–U+0E4E) — the shape of a correctable word. Non-Thai tokens
// (whitespace, punctuation, Latin, digits) are never sent to correction.
fn is_thai_char(c: char) -> bool {
    ('\u{0E01}'..='\u{0E4E}').contains(&c)
}
fn is_thai_word(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_thai_char)
}

const BIGRAM_BETA: f64 = 0.7; // Jelinek-Mercer weight on the bigram MLE vs the unigram backoff

/// A character trie over the dictionary, searched within a bounded edit distance. Walking the trie
/// shares the DP work across common prefixes and prunes whole subtrees once a row's minimum exceeds
/// the budget — far faster than scanning every word. Produces the same (word, distance) set as a
/// brute-force bounded Damerau-Levenshtein (optimal string alignment, incl. adjacent transposition).
struct DictTrie {
    children: Vec<Vec<(char, u32)>>, // node id -> [(edge char, child node id)]
    word: Vec<i32>,                  // node id -> dictionary word index, or -1 if not a word end
}

impl DictTrie {
    fn build(word_chars: &[Vec<char>]) -> Self {
        let mut children: Vec<Vec<(char, u32)>> = vec![Vec::new()];
        let mut word: Vec<i32> = vec![-1];
        for (wi, chars) in word_chars.iter().enumerate() {
            let mut node = 0usize;
            for &c in chars {
                let next = children[node].iter().find(|(ch, _)| *ch == c).map(|&(_, n)| n as usize);
                node = next.unwrap_or_else(|| {
                    let id = children.len();
                    children.push(Vec::new());
                    word.push(-1);
                    children[node].push((c, id as u32));
                    id
                });
            }
            word[node] = wi as i32;
        }
        DictTrie { children, word }
    }

    /// Dictionary words within `max` edits of `query`, as (word index, distance).
    fn search(&self, query: &[char], max: usize) -> Vec<(usize, usize)> {
        let qlen = query.len();
        // Depth-indexed DP rows, reused across the whole walk (no per-node allocation): `rows[d]` is
        // the row after matching d dictionary characters, grown lazily to the deepest path taken.
        let mut rows: Vec<Vec<usize>> = vec![(0..=qlen).collect()];
        let mut out = Vec::new();
        self.walk(0, 0, None, query, max, &mut rows, &mut out);
        out
    }

    // At `depth`, `rows[depth]` is this node's row and `rows[depth-1]` the parent's (for
    // transposition); `parent_char` is the edge char into this node. Each child fills `rows[depth+1]`.
    fn walk(
        &self,
        node: usize,
        depth: usize,
        parent_char: Option<char>,
        query: &[char],
        max: usize,
        rows: &mut Vec<Vec<usize>>,
        out: &mut Vec<(usize, usize)>,
    ) {
        let qlen = query.len();
        if rows.len() <= depth + 1 {
            rows.push(vec![0usize; qlen + 1]);
        }
        for idx in 0..self.children[node].len() {
            let (c, child) = self.children[node][idx];
            let row_min = {
                // Disjoint borrows: read rows[depth] (prev) & rows[depth-1] (prev2), write rows[depth+1].
                let (head, tail) = rows.split_at_mut(depth + 1);
                let prev = &head[depth];
                let prev2 = &head[depth.saturating_sub(1)]; // only read when parent_char is Some (depth>=1)
                let cur = &mut tail[0];
                cur[0] = prev[0] + 1;
                let mut row_min = cur[0];
                for j in 1..=qlen {
                    let cost = if c == query[j - 1] { 0 } else { 1 };
                    let mut v = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
                    if j >= 2 && c == query[j - 2] && parent_char == Some(query[j - 1]) {
                        v = v.min(prev2[j - 2] + 1);
                    }
                    cur[j] = v;
                    if v < row_min {
                        row_min = v;
                    }
                }
                row_min
            };
            if row_min > max {
                continue;
            }
            let child = child as usize;
            if self.word[child] >= 0 && rows[depth + 1][qlen] <= max {
                out.push((self.word[child] as usize, rows[depth + 1][qlen]));
            }
            self.walk(child, depth + 1, Some(c), query, max, rows, out);
        }
    }
}

pub struct SpellEngine {
    words: Vec<String>,       // the dictionary, indexable
    word_freq: Vec<u32>,      // parallel unigram counts (0 if unseen in the LM)
    trie: DictTrie,           // dictionary trie for bounded edit-distance candidate search
    membership: FxHashSet<String>, // fast "is this exactly a dictionary word?"
    // Aligned bigram model for correct_sent context scoring (see bigram_logp). Always loaded so
    // correct_sent covers the thai_words vocabulary the LST20 bigram lacks.
    align_unigram: rustc_hash::FxHashMap<String, u32>, // aligned unigram counts (bigram backoff)
    align_total: f64,                                  // sum of aligned unigram counts
    bigram: rustc_hash::FxHashMap<String, u32>,        // aligned bigrams, keyed "w1\u{1f}w2"
}

impl SpellEngine {
    /// Load the compiled-in dictionary and unigram frequencies. Called once via the process-global
    /// `OnceLock` in lib.rs.
    pub fn bootstrap() -> Self {
        let words: Vec<String> = include_str!("../assets/spell_words.txt")
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();
        let word_chars: Vec<Vec<char>> = words.iter().map(|w| w.chars().collect()).collect();
        let membership: FxHashSet<String> = words.iter().cloned().collect();

        // Ranking-frequency source, selectable at runtime (both tables are embedded):
        //   default / "aligned" — spell_unigrams_aligned.txt, counts over a corpus maximal-matched
        //                         to thai_words so EVERY dictionary word gets a real frequency. This
        //                         is the default because the corrector's candidates come from
        //                         thai_words, and the LST20 table simply lacks many of them (e.g.
        //                         ความสามารถ). Corpus-derived ("contaminated") — a ranking source
        //                         only; the candidate dictionary stays the clean thai_words. See tools/.
        //   "lst20"             — kn_words_unigrams.txt, the shipped LST20 LM. Cleaner counts, but
        //                         its word inventory diverges from thai_words. Opt-out for the pure
        //                         version / A-B comparison.
        // Format is "<word>\t<count>[\t...]" for both; only the first two columns are read.
        let freq_source = match std::env::var("THAPTHIM_SPELL_FREQ").ok().as_deref() {
            Some("lst20") => include_str!("../assets/kn_words_unigrams.txt"),
            _ => include_str!("../assets/spell_unigrams_aligned.txt"),
        };
        let mut freq_of: rustc_hash::FxHashMap<&str, u32> = rustc_hash::FxHashMap::default();
        for line in freq_source.lines() {
            let mut it = line.split('\t');
            if let (Some(w), Some(c)) = (it.next(), it.next()) {
                if let Ok(n) = c.parse::<u32>() {
                    freq_of.insert(w, n);
                }
            }
        }
        let word_freq: Vec<u32> = words
            .iter()
            .map(|w| freq_of.get(w.as_str()).copied().unwrap_or(0))
            .collect();

        let trie = DictTrie::build(&word_chars);

        // Aligned bigram model (always loaded, independent of the unigram-ranking selector): aligned
        // unigram counts for the backoff, and the aligned bigram counts. Both are thai_words-vocabulary.
        let mut align_unigram: rustc_hash::FxHashMap<String, u32> = rustc_hash::FxHashMap::default();
        let mut align_total: u64 = 0;
        for line in include_str!("../assets/spell_unigrams_aligned.txt").lines() {
            let mut it = line.split('\t');
            if let (Some(w), Some(c)) = (it.next(), it.next()) {
                if let Ok(n) = c.parse::<u32>() {
                    align_unigram.insert(w.to_string(), n);
                    align_total += n as u64;
                }
            }
        }
        let mut bigram: rustc_hash::FxHashMap<String, u32> = rustc_hash::FxHashMap::default();
        for line in include_str!("../assets/spell_bigrams_aligned.txt").lines() {
            let mut it = line.split('\t');
            if let (Some(a), Some(b), Some(c)) = (it.next(), it.next(), it.next()) {
                if let Ok(n) = c.parse::<u32>() {
                    bigram.insert(format!("{a}\u{1f}{b}"), n);
                }
            }
        }

        SpellEngine {
            words,
            word_freq,
            trie,
            membership,
            align_unigram,
            align_total: align_total.max(1) as f64,
            bigram,
        }
    }

    /// Kneser-Ney-free bigram log-probability P(w2 | w1) from the aligned counts, Jelinek-Mercer
    /// interpolated with the aligned unigram backoff so an unseen pair still scores (and w1 = "" acts
    /// as sentence start). Used by correct_sentence for context scoring over the thai_words vocabulary.
    pub fn bigram_logp(&self, w1: &str, w2: &str) -> f64 {
        let p_uni = (self.align_unigram.get(w2).copied().unwrap_or(0).max(1) as f64) / self.align_total;
        let c1 = self.align_unigram.get(w1).copied().unwrap_or(0) as f64;
        let mle = if c1 > 0.0 {
            self.bigram.get(&format!("{w1}\u{1f}{w2}")).copied().unwrap_or(0) as f64 / c1
        } else {
            0.0
        };
        (BIGRAM_BETA * mle + (1.0 - BIGRAM_BETA) * p_uni + 1e-12).ln()
    }

    /// Is `raw` a correctly-spelled word (a dictionary entry, after normalization)?
    pub fn is_word(&self, raw: &str) -> bool {
        let norm = crate::normalize::std_normalize(raw);
        !norm.is_empty() && self.membership.contains(&norm)
    }

    /// Ranked correction candidates for `raw` (best first), at most `top_n`. Empty if none within
    /// `max_edits`.
    pub fn suggest(&self, raw: &str, max_edits: usize, top_n: usize) -> Vec<String> {
        let norm = crate::normalize::std_normalize(raw);
        let query: Vec<char> = norm.chars().collect();
        if query.is_empty() {
            return Vec::new();
        }
        let mut scored: Vec<(f64, usize)> = self
            .candidates(&query, max_edits)
            .into_iter()
            .map(|(i, d)| (self.score(i, d), i))
            .collect();
        // Descending score; break ties by word index so the order is deterministic.
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });
        scored.into_iter().take(top_n).map(|(_, i)| self.words[i].clone()).collect()
    }

    /// Best-effort single correction. A valid word is returned unchanged (normalized); an unknown
    /// word becomes its top-ranked candidate, or is left unchanged if nothing is within `max_edits`.
    /// (A confidence gate to protect valid out-of-dictionary words — names, slang — is a later step.)
    pub fn correct(&self, raw: &str, max_edits: usize) -> String {
        let norm = crate::normalize::std_normalize(raw);
        if norm.is_empty() || self.membership.contains(&norm) {
            return norm;
        }
        self.suggest(&norm, max_edits, 1).into_iter().next().unwrap_or(norm)
    }

    fn score(&self, word_idx: usize, dist: usize) -> f64 {
        (self.word_freq[word_idx].max(1) as f64).log10() - LAMBDA * dist as f64
    }

    /// Dictionary words within `max` edits of `query`, as (word index, distance), via the trie.
    fn candidates(&self, query: &[char], max: usize) -> Vec<(usize, usize)> {
        self.trie.search(query, max)
    }

    /// The candidates a single token contributes to the sentence lattice: (surface, edit distance).
    /// Always includes the token itself at cost 0 (so it can be kept). Only an out-of-dictionary,
    /// Thai-word-shaped token additionally gets its top-N near-matches — valid words, whitespace,
    /// punctuation and non-Thai tokens are left with just the keep option, so correct text and
    /// structure are preserved.
    fn token_candidates(&self, token: &str, max_edits: usize) -> Vec<(String, usize)> {
        let mut cands = vec![(token.to_string(), 0usize)];
        let norm = crate::normalize::std_normalize(token);
        if norm.is_empty() || self.membership.contains(&norm) || !is_thai_word(&norm) {
            return cands;
        }
        let query: Vec<char> = norm.chars().collect();
        let mut scored: Vec<(f64, usize, usize)> = self
            .candidates(&query, max_edits)
            .into_iter()
            .map(|(idx, dist)| (self.score(idx, dist), idx, dist))
            .collect();
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(a.2.cmp(&b.2))
        });
        for (_, idx, dist) in scored.into_iter().take(SUGGEST_TOP_N) {
            cands.push((self.words[idx].clone(), dist));
        }
        cands
    }

    /// Context-aware correction of a pre-segmented token sequence. A bigram Viterbi chooses the
    /// sequence maximizing Σ bigram_logprob(prev→cur) − λ·Σ edit_cost, where each position's options
    /// come from `token_candidates`. `bigram` supplies the KN word-layer log-prob P(cur | prev); an
    /// empty `prev` marks sentence start (the LM backs off to a unigram-like prior there). Returns
    /// exactly one corrected surface per input token.
    pub fn correct_sentence(
        &self,
        tokens: &[&str],
        max_edits: usize,
        sent_lambda: f64,
        bigram: impl Fn(&str, &str) -> f64,
    ) -> Vec<String> {
        if tokens.is_empty() {
            return Vec::new();
        }
        let cand_lists: Vec<Vec<(String, usize)>> =
            tokens.iter().map(|&t| self.token_candidates(t, max_edits)).collect();
        let n = tokens.len();

        // Viterbi forward. `back[i][j]` = best predecessor candidate index for candidate j at pos i.
        let mut back: Vec<Vec<usize>> = Vec::with_capacity(n);
        let mut prev_scores: Vec<f64> = cand_lists[0]
            .iter()
            .map(|(surf, dist)| bigram("", surf) - sent_lambda * *dist as f64)
            .collect();
        back.push(vec![usize::MAX; cand_lists[0].len()]);

        for i in 1..n {
            let mut cur_scores = Vec::with_capacity(cand_lists[i].len());
            let mut cur_back = Vec::with_capacity(cand_lists[i].len());
            for (surf, dist) in &cand_lists[i] {
                let node = -sent_lambda * *dist as f64;
                let mut best = f64::NEG_INFINITY;
                let mut best_k = 0usize;
                for (k, (psurf, _)) in cand_lists[i - 1].iter().enumerate() {
                    let s = prev_scores[k] + bigram(psurf, surf);
                    if s > best {
                        best = s;
                        best_k = k;
                    }
                }
                cur_scores.push(best + node);
                cur_back.push(best_k);
            }
            prev_scores = cur_scores;
            back.push(cur_back);
        }

        // Backtrack from the best final candidate.
        let mut j = prev_scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        let mut path = vec![0usize; n];
        for i in (0..n).rev() {
            path[i] = j;
            if i > 0 {
                j = back[i][j];
            }
        }
        (0..n).map(|i| cand_lists[i][path[i]].0.clone()).collect()
    }
}
