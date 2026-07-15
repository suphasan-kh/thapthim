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

pub struct SpellEngine {
    words: Vec<String>,       // the dictionary, indexable
    word_chars: Vec<Vec<char>>, // parallel char arrays for the edit-distance scan
    word_freq: Vec<u32>,      // parallel unigram counts (0 if unseen in the LM)
    by_len: Vec<Vec<u32>>,    // word indices grouped by char length, for the length prefilter
    membership: FxHashSet<String>, // fast "is this exactly a dictionary word?"
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

        // Unigram counts: "<word>\t<count>\t<cont_count>". Only the first two columns are used.
        let mut freq_of: rustc_hash::FxHashMap<&str, u32> = rustc_hash::FxHashMap::default();
        for line in include_str!("../assets/kn_words_unigrams.txt").lines() {
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

        let max_len = word_chars.iter().map(|c| c.len()).max().unwrap_or(0);
        let mut by_len: Vec<Vec<u32>> = vec![Vec::new(); max_len + 1];
        for (i, c) in word_chars.iter().enumerate() {
            by_len[c.len()].push(i as u32);
        }

        SpellEngine { words, word_chars, word_freq, by_len, membership }
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

    /// Dictionary words within `max` edits of `query`, as (word index, distance). Only lengths
    /// within `max` of the query length are examined.
    fn candidates(&self, query: &[char], max: usize) -> Vec<(usize, usize)> {
        let ql = query.len();
        let lo = ql.saturating_sub(max);
        let hi = (ql + max).min(self.by_len.len().saturating_sub(1));
        let mut out = Vec::new();
        for len in lo..=hi {
            for &wi in &self.by_len[len] {
                let wi = wi as usize;
                if let Some(d) = damerau_levenshtein_bounded(query, &self.word_chars[wi], max) {
                    out.push((wi, d));
                }
            }
        }
        out
    }
}

/// Bounded Damerau-Levenshtein (optimal string alignment with adjacent transposition) on char
/// slices. Returns `None` as soon as it can prove the distance exceeds `max`, so most far-apart
/// words are rejected after their first row.
fn damerau_levenshtein_bounded(a: &[char], b: &[char], max: usize) -> Option<usize> {
    let (la, lb) = (a.len(), b.len());
    if la.abs_diff(lb) > max {
        return None;
    }
    let mut prev2: Vec<usize> = vec![0; lb + 1];
    let mut prev: Vec<usize> = (0..=lb).collect();
    let mut cur: Vec<usize> = vec![0; lb + 1];
    for i in 1..=la {
        cur[0] = i;
        let mut row_min = i;
        for j in 1..=lb {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            let mut v = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(prev2[j - 2] + 1);
            }
            cur[j] = v;
            if v < row_min {
                row_min = v;
            }
        }
        if row_min > max {
            return None;
        }
        // Rotate the three rows: prev2 <- prev, prev <- cur, cur <- (old prev2, reused as scratch).
        std::mem::swap(&mut prev2, &mut prev);
        std::mem::swap(&mut prev, &mut cur);
    }
    let d = prev[lb];
    if d <= max { Some(d) } else { None }
}
