// ext/thapthim/src/lattice/decode.rs
//
// The lattice itself: build the TCC grid, gather grid-aligned dictionary candidates plus the
// TCC fallback chain, run the exact bigram Viterbi, and expose the public `segment_*` entry
// points that assemble these into packed token streams.
use super::*;
use std::collections::HashSet;
use rustc_hash::FxHashMap;
use daachorse::CharwiseDoubleArrayAhoCorasick;

impl RuntimeEngine {
    /// TCC byte boundaries form the atomic grid. Every word/syllable candidate must begin and
    /// end on one of these, which is what guarantees the nested word ⊂ syllable ⊂ TCC invariant.
    /// Returns the boundary positions, a set for O(1) membership, and a position→index map for
    /// measuring span length in TCCs.
    fn tcc_grid(&self, text: &str) -> (Vec<usize>, HashSet<usize>, FxHashMap<usize, usize>) {
        let positions = self.tcc.find_byte_positions(text);
        let boundary: HashSet<usize> = positions.iter().copied().collect();
        let mut byte_to_idx: FxHashMap<usize, usize> = FxHashMap::default();
        for (i, &p) in positions.iter().enumerate() {
            byte_to_idx.insert(p, i);
        }
        (positions, boundary, byte_to_idx)
    }

    /// Overlapping dictionary matches from a charwise trie, kept only when grid-aligned and
    /// within `max_word_tcc` TCC clusters. (The reference paper's word-length figure is in
    /// characters; this cap is in TCC clusters and was tuned empirically — LST20 plateaus at
    /// 10–12 TCC; see THAPTHIM_MAX_WORD_TCC.)
    fn dict_candidates(
        &self,
        trie: &CharwiseDoubleArrayAhoCorasick<usize>,
        text: &str,
        tier: LatticeTier,
        boundary: &HashSet<usize>,
        byte_to_idx: &FxHashMap<usize, usize>,
    ) -> Vec<Cand> {
        // `0` disables the cap (accept any dictionary match); see THAPTHIM_MAX_WORD_TCC.
        let max_tcc = if self.max_word_tcc == 0 { usize::MAX } else { self.max_word_tcc };
        let mut cands = Vec::new();
        for m in trie.find_overlapping_iter(text) {
            let (s, e) = (m.start(), m.end());
            if boundary.contains(&s) && boundary.contains(&e) {
                let len_tcc = byte_to_idx[&e] - byte_to_idx[&s];
                if len_tcc >= 1 && len_tcc <= max_tcc {
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
    /// `cand_ids[i]` is `cands[i].text`'s token id in `lm_tier`'s layer, precomputed once by the
    /// caller (a token is a predecessor and a successor of many edges; resolving its id per node
    /// instead of per edge keeps the inner loop free of String hashing).
    fn decode_best_path(
        &self,
        cands: &[Cand],
        cand_ids: &[Option<u32>],
        rs: usize,
        re: usize,
        lm_tier: &LatticeTier,
    ) -> Vec<Cand> {
        if rs >= re {
            return Vec::new();
        }
        let layer = self.layer(lm_tier);
        let space_id = layer.token_id.get(" ").copied(); // initial context, resolved once

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

        // Candidate indices are dense, so dp/bp/reached are plain Vecs (no per-edge usize hashing).
        // `reached` is a separate flag rather than a dp sentinel: `score` can legitimately return
        // NEG_INFINITY (a seen token with no followers and a zero bigram count → ln(0)), so a
        // score value cannot stand in for "was this node reached".
        let mut dp = vec![f64::NEG_INFINITY; cands.len()];
        let mut bp: Vec<Option<usize>> = vec![None; cands.len()];
        let mut reached = vec![false; cands.len()];

        for &i in &order {
            let c = &cands[i];
            let best: Option<(f64, Option<usize>)> = if c.start == rs {
                Some((layer.score(space_id, cand_ids[i], self.oov_penalty), None))
            } else {
                let mut b: Option<(f64, Option<usize>)> = None;
                if let Some(prevs) = ending_at.get(&c.start) {
                    for &p in prevs {
                        if reached[p] {
                            let s = dp[p] + layer.score(cand_ids[p], cand_ids[i], self.oov_penalty);
                            if b.is_none_or(|(bs, _)| s > bs) {
                                b = Some((s, Some(p)));
                            }
                        }
                    }
                }
                b
            };
            if let Some((s, prev)) = best {
                dp[i] = s;
                bp[i] = prev;
                reached[i] = true;
            }
        }

        let mut best_end: Option<(f64, usize)> = None;
        if let Some(ends) = ending_at.get(&re) {
            for &i in ends {
                if reached[i]
                    && best_end.is_none_or(|(bs, _)| dp[i] > bs) {
                        best_end = Some((dp[i], i));
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
            cur = bp[i];
        }
        path.reverse();
        path
    }

    /// Resolve every candidate's token id in `lm_tier`'s layer, once, for the decode hot path.
    fn candidate_ids(&self, cands: &[Cand], lm_tier: &LatticeTier) -> Vec<Option<u32>> {
        let layer = self.layer(lm_tier);
        cands.iter().map(|c| layer.token_id.get(c.text.as_str()).copied()).collect()
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
        let word_ids = self.candidate_ids(&word_cands, &LatticeTier::Word);
        let word_path = self.decode_best_path(&word_cands, &word_ids, 0, byte_len, &LatticeTier::Word);

        // Coalesce consecutive TCC-fallback nodes into maximal OOV spans.
        let mut spans: Vec<(usize, usize, bool)> = Vec::new();
        for c in &word_path {
            match c.tier {
                LatticeTier::Word => spans.push((c.start, c.end, true)),
                _ => {
                    if let Some(last) = spans.last_mut()
                        && !last.2 && last.1 == c.start {
                            last.1 = c.end;
                            continue;
                        }
                    spans.push((c.start, c.end, false));
                }
            }
        }

        // Baseline token list: dictionary words verbatim, OOV runs syllabified (syllable cands
        // and their precomputed ids built lazily — only if some span actually needs them).
        let mut syl: Option<(Vec<Cand>, Vec<Option<u32>>)> = None;
        let mut toks: Vec<(usize, usize, LatticeTier)> = Vec::new();
        for (s, e, is_word) in spans {
            if is_word {
                toks.push((s, e, LatticeTier::Word));
            } else {
                let entry = syl.get_or_insert_with(|| {
                    let mut c = self.dict_candidates(
                        &self.syllable_trie,
                        text,
                        LatticeTier::Syllable,
                        &boundary,
                        &byte_to_idx,
                    );
                    c.extend(fallback.iter().cloned());
                    let ids = self.candidate_ids(&c, &LatticeTier::Syllable);
                    (c, ids)
                });
                for sy in &self.decode_best_path(&entry.0, &entry.1, s, e, &LatticeTier::Syllable) {
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
        let cand_ids = self.candidate_ids(&cands, &LatticeTier::Syllable);

        self.decode_best_path(&cands, &cand_ids, 0, byte_len, &LatticeTier::Syllable)
            .iter()
            .map(|c| pack(c.start, c.end, tier_flag(&c.tier)))
            .collect()
    }
}
