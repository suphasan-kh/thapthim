// ext/thapthim/src/lattice/decode.rs
//
// The lattice itself: build the TCC grid, gather grid-aligned dictionary candidates plus the
// TCC fallback chain, run the exact bigram Viterbi, and expose the public `segment_*` entry
// points that assemble these into packed token streams.
use super::*;
use std::collections::HashSet;
use rustc_hash::FxHashMap;
use daachorse::CharwiseDoubleArrayAhoCorasick;
use crate::tcc::TccSegmenter;

impl RuntimeEngine {
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

        // Baseline token list: dictionary words verbatim, OOV runs syllabified (syllable cands
        // built lazily — only if some span actually needs them).
        let mut syl_cands: Option<Vec<Cand>> = None;
        let mut toks: Vec<(usize, usize, LatticeTier)> = Vec::new();
        for (s, e, is_word) in spans {
            if is_word {
                toks.push((s, e, LatticeTier::Word));
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
                    toks.push((sy.start, sy.end, sy.tier.clone()));
                }
            }
        }

        // Branching-entropy OOV-merge post-pass over the full path. Fixes over-splits where short
        // dictionary words tile one unknown word (ลัน|ดัน → ลันดัน). No-op when disabled.
        let toks = if self.be_threshold > 0.0 {
            self.merge_short_runs(text, &toks, &byte_to_idx)
        } else {
            toks
        };

        toks.iter().map(|(s, e, t)| pack(*s, *e, tier_flag(t))).collect()
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
