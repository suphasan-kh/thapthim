// ext/thapthim/src/lattice.rs
use serde::{Serialize, Deserialize};
use rustc_hash::FxHashMap;
use std::collections::HashSet;
use daachorse::CharwiseDoubleArrayAhoCorasick;
use crate::tcc::TccSegmenter;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LayerCounts {
    pub unigrams: FxHashMap<String, (usize, usize)>,
    pub bigrams: FxHashMap<String, usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MasterLanguageModel {
    pub words: LayerCounts,
    pub syllables: LayerCounts,
    pub tccs: LayerCounts,
}

/// Statistics derived once at bootstrap so the Viterbi hot path never scans the n-gram
/// tables. `*_followers[w1]` is the number of distinct types observed after `w1`
/// (the Kneser-Ney `N₁₊(w1 •)` term), which previously cost a full bigram-table scan per edge.
struct PrecomputedStats {
    word_followers: FxHashMap<String, usize>,
    syllable_followers: FxHashMap<String, usize>,
    tcc_followers: FxHashMap<String, usize>,
}

pub struct RuntimeEngine {
    pub word_trie: CharwiseDoubleArrayAhoCorasick<usize>,
    pub syllable_trie: CharwiseDoubleArrayAhoCorasick<usize>,
    pub lm: MasterLanguageModel,
    stats: PrecomputedStats,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LatticeTier {
    Word,
    Syllable,
    Tcc,
}

/// A grid-aligned candidate span (dictionary word/syllable, or a single-TCC fallback).
#[derive(Clone, Debug)]
struct Cand {
    start: usize,
    end: usize,
    text: String,
    tier: LatticeTier,
}

fn tier_flag(tier: &LatticeTier) -> u64 {
    match tier {
        LatticeTier::Word => 1,
        LatticeTier::Syllable => 2,
        LatticeTier::Tcc => 3,
    }
}

/// Pack [ Start: 32 bits | Length: 24 bits | Tier: 8 bits ] into a u64. Start/Length are byte
/// offsets into the source text; Tier is 1 Word / 2 Syllable / 3 Tcc.
fn pack(start: usize, end: usize, flag: u64) -> u64 {
    ((start as u64) << 32) | (((end - start) as u64) << 8) | flag
}

impl RuntimeEngine {
    pub fn bootstrap() -> Self {
        let words_raw = include_str!("../assets/master_words_vocab.txt");
        let mut word_patterns = Vec::new();
        for (idx, line) in words_raw.lines().enumerate() {
            if !line.is_empty() {
                word_patterns.push((line.to_string(), idx));
            }
        }
        let word_trie = CharwiseDoubleArrayAhoCorasick::with_values(word_patterns).unwrap();

        let syllables_raw = include_str!("../assets/master_syllables_vocab.txt");
        let mut syllable_patterns = Vec::new();
        for (idx, line) in syllables_raw.lines().enumerate() {
            if !line.is_empty() {
                syllable_patterns.push((line.to_string(), idx));
            }
        }
        let syllable_trie = CharwiseDoubleArrayAhoCorasick::with_values(syllable_patterns).unwrap();

        let lm_bytes = include_bytes!("../assets/joint_lm.bin");
        let lm: MasterLanguageModel = bincode::deserialize(lm_bytes)
            .expect("Critical error reading joint_lm.bin. Structure formats mismatch.");

        let stats = PrecomputedStats {
            word_followers: Self::compute_follower_types(&lm.words),
            syllable_followers: Self::compute_follower_types(&lm.syllables),
            tcc_followers: Self::compute_follower_types(&lm.tccs),
        };

        RuntimeEngine { word_trie, syllable_trie, lm, stats }
    }

    /// Counts the distinct followers of each `w1` in a layer's bigram table in a single pass.
    /// Each bigram key `"w1\tw2"` is unique, so tallying keys per `w1` yields `N₁₊(w1 •)`.
    fn compute_follower_types(layer: &LayerCounts) -> FxHashMap<String, usize> {
        let mut followers: FxHashMap<String, usize> = FxHashMap::default();
        for key in layer.bigrams.keys() {
            if let Some((w1, _)) = key.split_once('\t') {
                *followers.entry(w1.to_string()).or_insert(0) += 1;
            }
        }
        followers
    }

    fn follower_types_for(&self, lm_tier: &LatticeTier) -> &FxHashMap<String, usize> {
        match lm_tier {
            LatticeTier::Word => &self.stats.word_followers,
            LatticeTier::Syllable => &self.stats.syllable_followers,
            LatticeTier::Tcc => &self.stats.tcc_followers,
        }
    }

    /// TCC byte boundaries form the atomic grid. Every word/syllable candidate must begin and
    /// end on one of these, which is what guarantees the nested word ⊂ syllable ⊂ TCC invariant.
    /// Returns the boundary positions, a set for O(1) membership, and a position→index map for
    /// measuring span length in TCCs.
    fn tcc_grid(&self, text: &str) -> (Vec<usize>, HashSet<usize>, FxHashMap<usize, usize>) {
        let positions = TccSegmenter::new().find_byte_positions(text);
        let boundary: HashSet<usize> = positions.iter().copied().collect();
        let mut byte_to_idx: FxHashMap<usize, usize> = FxHashMap::default();
        for (i, &p) in positions.iter().enumerate() {
            byte_to_idx.insert(p, i);
        }
        (positions, boundary, byte_to_idx)
    }

    /// Overlapping dictionary matches from a charwise trie, kept only when grid-aligned and
    /// within the maximum word length (≈12 TCCs — the empirical 80th-percentile Thai word).
    fn dict_candidates(
        &self,
        trie: &CharwiseDoubleArrayAhoCorasick<usize>,
        text: &str,
        tier: LatticeTier,
        boundary: &HashSet<usize>,
        byte_to_idx: &FxHashMap<usize, usize>,
    ) -> Vec<Cand> {
        const MAX_WORD_TCC: usize = 12;
        let mut cands = Vec::new();
        for m in trie.find_overlapping_iter(text) {
            let (s, e) = (m.start(), m.end());
            if boundary.contains(&s) && boundary.contains(&e) {
                let len_tcc = byte_to_idx[&e] - byte_to_idx[&s];
                if len_tcc >= 1 && len_tcc <= MAX_WORD_TCC {
                    cands.push(Cand {
                        start: s,
                        end: e,
                        text: text[s..e].to_string(),
                        tier: tier.clone(),
                    });
                }
            }
        }
        cands
    }

    /// One node per TCC: the always-present fallback chain guaranteeing a complete path through
    /// any region (OOV spans in the word decode, and the floor for the syllable decode).
    fn tcc_fallback(&self, text: &str, positions: &[usize]) -> Vec<Cand> {
        let mut cands = Vec::new();
        for w in positions.windows(2) {
            let (s, e) = (w[0], w[1]);
            cands.push(Cand {
                start: s,
                end: e,
                text: text[s..e].to_string(),
                tier: LatticeTier::Tcc,
            });
        }
        cands
    }

    /// Exact first-order (bigram) Viterbi over candidates confined to `[rs, re)`. The DP state is
    /// the node itself, so the previous token is captured exactly. Every transition is scored
    /// under `lm_tier`'s language model regardless of a node's own tier (a TCC fallback in the
    /// word decode is therefore priced as an OOV word). Never returns empty while the TCC
    /// fallback chain is present.
    fn decode_best_path(&self, cands: &[Cand], rs: usize, re: usize, lm_tier: &LatticeTier) -> Vec<Cand> {
        if rs >= re {
            return Vec::new();
        }

        let in_region: Vec<usize> = (0..cands.len())
            .filter(|&i| cands[i].start >= rs && cands[i].end <= re)
            .collect();

        let mut ending_at: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
        for &i in &in_region {
            ending_at.entry(cands[i].end).or_default().push(i);
        }

        // A node's predecessors end at its start (so they start strictly earlier); processing in
        // ascending start order therefore guarantees predecessors are scored first.
        let mut order = in_region.clone();
        order.sort_by_key(|&i| cands[i].start);

        let mut dp: FxHashMap<usize, f64> = FxHashMap::default();
        let mut bp: FxHashMap<usize, Option<usize>> = FxHashMap::default();

        for &i in &order {
            let c = &cands[i];
            let best: Option<(f64, Option<usize>)> = if c.start == rs {
                Some((self.score_transition(lm_tier, " ", &c.text), None))
            } else {
                let mut b: Option<(f64, Option<usize>)> = None;
                if let Some(prevs) = ending_at.get(&c.start) {
                    for &p in prevs {
                        if let Some(&ps) = dp.get(&p) {
                            let s = ps + self.score_transition(lm_tier, &cands[p].text, &c.text);
                            if b.map_or(true, |(bs, _)| s > bs) {
                                b = Some((s, Some(p)));
                            }
                        }
                    }
                }
                b
            };
            if let Some((s, prev)) = best {
                dp.insert(i, s);
                bp.insert(i, prev);
            }
        }

        let mut best_end: Option<(f64, usize)> = None;
        if let Some(ends) = ending_at.get(&re) {
            for &i in ends {
                if let Some(&s) = dp.get(&i) {
                    if best_end.map_or(true, |(bs, _)| s > bs) {
                        best_end = Some((s, i));
                    }
                }
            }
        }

        let mut path = Vec::new();
        let mut cur = best_end.map(|(_, i)| i);
        while let Some(i) = cur {
            let c = &cands[i];
            path.push(Cand {
                start: c.start,
                end: c.end,
                text: c.text.clone(),
                tier: c.tier.clone(),
            });
            cur = *bp.get(&i).unwrap_or(&None);
        }
        path.reverse();
        path
    }

    /// Kneser-Ney–smoothed bigram log-probability under `lm_tier`'s layer. When `w1` is unseen
    /// by that layer (common for dictionary-only words), it backs off to `w2`'s continuation
    /// probability rather than a flat floor, so the score still reflects how connectable `w2` is.
    pub fn score_transition(&self, lm_tier: &LatticeTier, w1: &str, w2: &str) -> f64 {
        let layer = match lm_tier {
            LatticeTier::Word => &self.lm.words,
            LatticeTier::Syllable => &self.lm.syllables,
            LatticeTier::Tcc => &self.lm.tccs,
        };

        let bigram_key = format!("{}\t{}", w1, w2);
        let count_bi = layer.bigrams.get(&bigram_key).cloned().unwrap_or(0);
        let count_w1 = layer.unigrams.get(w1).map(|v| v.0).unwrap_or(0);
        let d = 0.75;
        let total_bigram_types = layer.bigrams.len() as f64;

        if count_w1 > 0 {
            // Precomputed at bootstrap (was a full bigram-table scan per edge).
            let unique_following_w1 = self.follower_types_for(lm_tier).get(w1).copied().unwrap_or(0);

            let lambda = (d / count_w1 as f64) * unique_following_w1 as f64;
            let (unigram_count, preceding_contexts) =
                layer.unigrams.get(w2).cloned().unwrap_or((0, 0));

            let p_continuation = if preceding_contexts > 0 {
                preceding_contexts as f64 / total_bigram_types
            } else {
                (unigram_count.max(1) as f64) / total_bigram_types
            };

            let primary_term = ((count_bi as f64 - d).max(0.0)) / count_w1 as f64;
            (primary_term + lambda * p_continuation).ln()
        } else {
            // w1 unseen by this layer: back off to w2's (add-one smoothed) continuation prob.
            let preceding_contexts = layer.unigrams.get(w2).map(|v| v.1).unwrap_or(0) as f64;
            ((preceding_contexts + 1.0) / (total_bigram_types + 1.0)).ln()
        }
    }

    /// Word segmentation (single word-LM Viterbi). Spans the dictionary cannot cover come back
    /// as a maximal OOV run which is then syllabified *only for those spans* — so when the input
    /// is fully in-vocabulary, no syllable work happens at all. Returns packed word tokens.
    pub fn segment_words(&self, text: &str) -> Vec<u64> {
        let byte_len = text.len();
        if byte_len == 0 {
            return Vec::new();
        }
        let (positions, boundary, byte_to_idx) = self.tcc_grid(text);
        if positions.len() < 2 {
            return Vec::new();
        }
        let fallback = self.tcc_fallback(text, &positions);

        let mut word_cands =
            self.dict_candidates(&self.word_trie, text, LatticeTier::Word, &boundary, &byte_to_idx);
        word_cands.extend(fallback.iter().cloned());
        let word_path = self.decode_best_path(&word_cands, 0, byte_len, &LatticeTier::Word);

        // Coalesce consecutive TCC-fallback nodes into maximal OOV spans.
        let mut spans: Vec<(usize, usize, bool)> = Vec::new();
        for c in &word_path {
            match c.tier {
                LatticeTier::Word => spans.push((c.start, c.end, true)),
                _ => {
                    if let Some(last) = spans.last_mut() {
                        if !last.2 && last.1 == c.start {
                            last.1 = c.end;
                            continue;
                        }
                    }
                    spans.push((c.start, c.end, false));
                }
            }
        }

        // Syllable candidates are built lazily — only if at least one OOV span needs them.
        let mut syl_cands: Option<Vec<Cand>> = None;
        let mut out = Vec::new();
        for (s, e, is_word) in spans {
            if is_word {
                out.push(pack(s, e, tier_flag(&LatticeTier::Word)));
            } else {
                let cands = syl_cands.get_or_insert_with(|| {
                    let mut c = self.dict_candidates(
                        &self.syllable_trie,
                        text,
                        LatticeTier::Syllable,
                        &boundary,
                        &byte_to_idx,
                    );
                    c.extend(fallback.iter().cloned());
                    c
                });
                for sy in &self.decode_best_path(&cands[..], s, e, &LatticeTier::Syllable) {
                    out.push(pack(sy.start, sy.end, tier_flag(&sy.tier)));
                }
            }
        }
        out
    }

    /// Syllable segmentation (single syllable-LM Viterbi over the whole text). Independent of the
    /// word decode and matches how the syllable bigrams were trained (flat sentence-level
    /// sequences that cross word boundaries). Returns packed syllable tokens.
    pub fn segment_syllables(&self, text: &str) -> Vec<u64> {
        let byte_len = text.len();
        if byte_len == 0 {
            return Vec::new();
        }
        let (positions, boundary, byte_to_idx) = self.tcc_grid(text);
        if positions.len() < 2 {
            return Vec::new();
        }

        let mut cands = self.dict_candidates(
            &self.syllable_trie,
            text,
            LatticeTier::Syllable,
            &boundary,
            &byte_to_idx,
        );
        cands.extend(self.tcc_fallback(text, &positions));

        self.decode_best_path(&cands, 0, byte_len, &LatticeTier::Syllable)
            .iter()
            .map(|c| pack(c.start, c.end, tier_flag(&c.tier)))
            .collect()
    }

    /// EXPERIMENT (A/B vs `segment_words`). Word segmentation where dictionary syllables are
    /// first-class candidates in the word lattice alongside words and the TCC floor — all scored
    /// under the WORD LM (one consistent space, so no cross-tier scale issue). Syllables can thus
    /// influence boundary placement, not just fill OOV spans after the fact. Risk: a known word
    /// may lose to its cheaper syllable pieces (over-segmentation), hence the measurement.
    pub fn segment_words_joint(&self, text: &str) -> Vec<u64> {
        let byte_len = text.len();
        if byte_len == 0 {
            return Vec::new();
        }
        let (positions, boundary, byte_to_idx) = self.tcc_grid(text);
        if positions.len() < 2 {
            return Vec::new();
        }

        let mut cands =
            self.dict_candidates(&self.word_trie, text, LatticeTier::Word, &boundary, &byte_to_idx);
        cands.extend(self.dict_candidates(
            &self.syllable_trie,
            text,
            LatticeTier::Syllable,
            &boundary,
            &byte_to_idx,
        ));
        cands.extend(self.tcc_fallback(text, &positions));

        self.decode_best_path(&cands, 0, byte_len, &LatticeTier::Word)
            .iter()
            .map(|c| pack(c.start, c.end, tier_flag(&c.tier)))
            .collect()
    }
}
