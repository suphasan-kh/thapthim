// ext/thapthim/src/lattice/decode.rs
//
// Word/syllable segmentation as the first instantiation of the generic grid lattice (grid.rs):
// build the TCC grid, gather grid-aligned dictionary candidates plus the TCC fallback chain, then
// run the shared `viterbi` under `BigramModel` — the Kneser-Ney bigram cost. The `segment_*` entry
// points assemble the resulting path into packed token streams.
use super::grid::{viterbi, Edge, LatticeModel};
use super::*;
use daachorse::CharwiseDoubleArrayAhoCorasick;

/// Sentinel in the byte→TCC-index array for a byte offset that is *not* a grid boundary.
/// `u32::MAX` is safe: a real index is `< positions.len()`, far below this on any real input.
pub(super) const NON_BOUNDARY: u32 = u32::MAX;

/// A token that is exactly one bare Thai consonant (U+0E01..=U+0E2E, nothing else). The word
/// vocabulary carries all 43 consonants as degenerate one-letter entries; inside an OOV region the
/// Viterbi will happily tile a transliteration with them (บ|ลั|ช), which both fragments the run and
/// leaks a sub-syllabic TCC into the word output. `segment_words` treats such a "word" as OOV
/// fallback so the whole run is syllabified as one. (`ณ`/`ธ` are genuine one-letter words, but an
/// *isolated* one becomes a length-1 OOV span and re-emerges as the identical surface token, so this
/// folding is boundary-neutral for them — see the call site.)
fn is_bare_consonant(s: &str) -> bool {
    let mut chars = s.chars();
    matches!((chars.next(), chars.next()), (Some(c), None) if ('\u{0E01}'..='\u{0E2E}').contains(&c))
}

/// The Kneser-Ney bigram cost over one LM layer — the segmentation instantiation of `LatticeModel`.
/// A node's context is its surface token's id in `layer` (resolved by slicing `text`, no owned
/// String per candidate); the transition is `RuntimeLayer::score`. `node_cost` stays the default
/// `0.0`, so the decoded path is bit-identical to the pre-refactor `decode_best_path`.
struct BigramModel<'a> {
    text: &'a str,
    layer: &'a RuntimeLayer,
    oov_penalty: f64,
    kn: f64,
}

impl LatticeModel for BigramModel<'_> {
    type Payload = LatticeTier;
    type Ctx = Option<u32>;

    fn start_ctx(&self) -> Option<u32> {
        self.layer.token_id.get(" ").copied()
    }

    fn contexts(&self, edges: &[Edge<LatticeTier>]) -> Vec<Option<u32>> {
        edges
            .iter()
            .map(|e| self.layer.token_id.get(&self.text[e.start..e.end]).copied())
            .collect()
    }

    fn transition(&self, prev: Option<u32>, cur: Option<u32>) -> f64 {
        self.layer.score(prev, cur, self.oov_penalty, self.kn)
    }
}

impl RuntimeEngine {
    /// The bigram cost model for `tier`'s LM layer, bound to `text` for the decode of one call.
    /// Cheap (four field copies); the per-node work happens in `BigramModel::contexts`.
    fn bigram_model<'a>(&'a self, text: &'a str, tier: &LatticeTier) -> BigramModel<'a> {
        BigramModel {
            text,
            layer: self.layer(tier),
            oov_penalty: self.oov_penalty,
            kn: self.kn_discount,
        }
    }

    /// TCC byte boundaries form the atomic grid. Every word/syllable candidate must begin and
    /// end on one of these, which is what guarantees the nested word ⊂ syllable ⊂ TCC invariant.
    /// Returns the boundary positions and a `byte offset → TCC index` array (length `text.len()+1`,
    /// `NON_BOUNDARY` where the offset is not a grid point). A flat array — not a hashmap — because
    /// `dict_candidates` probes it twice for *every* overlapping dictionary match (far more probes
    /// than there are boundaries), and an array index is a cache hit where a hash probe is not. The
    /// array doubles as the grid-membership test: a non-sentinel value *is* the boundary proof.
    fn tcc_grid(&self, text: &str) -> (Vec<usize>, Vec<u32>) {
        let positions = self.tcc.find_byte_positions(text);
        let mut byte_to_idx = vec![NON_BOUNDARY; text.len() + 1];
        for (i, &p) in positions.iter().enumerate() {
            byte_to_idx[p] = i as u32;
        }
        (positions, byte_to_idx)
    }

    /// Overlapping dictionary matches from a charwise trie, kept only when grid-aligned and
    /// within `max_word_tcc` TCC clusters. (The reference paper's word-length figure is in
    /// characters; this cap is in TCC clusters and was tuned empirically — LST20 plateaus at
    /// 10–12 TCC; see THAPTHIM_MAX_WORD_TCC.) Grid-alignment and span length are read from one
    /// `byte_to_idx` array index per endpoint: a non-`NON_BOUNDARY` value *is* the boundary proof.
    fn dict_candidates(
        &self,
        trie: &CharwiseDoubleArrayAhoCorasick<usize>,
        text: &str,
        tier: LatticeTier,
        byte_to_idx: &[u32],
    ) -> Vec<Edge<LatticeTier>> {
        // `0` disables the cap (accept any dictionary match); see THAPTHIM_MAX_WORD_TCC.
        let max_tcc = if self.max_word_tcc == 0 { usize::MAX } else { self.max_word_tcc };
        let mut cands = Vec::new();
        for m in trie.find_overlapping_iter(text) {
            let (s, e) = (m.start(), m.end());
            let (si, ei) = (byte_to_idx[s], byte_to_idx[e]);
            if si != NON_BOUNDARY && ei != NON_BOUNDARY {
                let len_tcc = (ei - si) as usize;
                if len_tcc >= 1 && len_tcc <= max_tcc {
                    cands.push(Edge { start: s, end: e, payload: tier.clone() });
                }
            }
        }
        cands
    }

    /// Like `dict_candidates`, but also returns each edge's word-LM context id resolved from the
    /// precomputed `word_dict_lm` table (the trie match's value is the dict line index) instead of
    /// hashing the surface string. Word decode only — the hot path; other passes use `contexts`.
    fn word_dict_candidates_ctx(
        &self,
        text: &str,
        byte_to_idx: &[u32],
    ) -> (Vec<Edge<LatticeTier>>, Vec<Option<u32>>) {
        let max_tcc = if self.max_word_tcc == 0 { usize::MAX } else { self.max_word_tcc };
        let mut cands = Vec::new();
        let mut ctx = Vec::new();
        for m in self.word_trie.find_overlapping_iter(text) {
            let (s, e) = (m.start(), m.end());
            let (si, ei) = (byte_to_idx[s], byte_to_idx[e]);
            if si != NON_BOUNDARY && ei != NON_BOUNDARY {
                let len_tcc = (ei - si) as usize;
                if len_tcc >= 1 && len_tcc <= max_tcc {
                    cands.push(Edge { start: s, end: e, payload: LatticeTier::Word });
                    ctx.push(self.word_dict_lm[m.value()]);
                }
            }
        }
        (cands, ctx)
    }

    /// One node per TCC: the always-present fallback chain guaranteeing a complete path through
    /// any region (OOV spans in the word decode, and the floor for the syllable decode).
    fn tcc_fallback(&self, positions: &[usize]) -> Vec<Edge<LatticeTier>> {
        positions
            .windows(2)
            .map(|w| Edge { start: w[0], end: w[1], payload: LatticeTier::Tcc })
            .collect()
    }

    /// Word segmentation (single word-LM Viterbi). Spans the dictionary cannot cover come back
    /// as a maximal OOV run which is then syllabified *only for those spans* — so when the input
    /// is fully in-vocabulary, no syllable work happens at all. Returns packed word tokens.
    pub fn segment_words(&self, text: &str) -> Vec<u64> {
        let byte_len = text.len();
        if byte_len == 0 {
            return Vec::new();
        }
        let (positions, byte_to_idx) = self.tcc_grid(text);
        if positions.len() < 2 {
            return Vec::new();
        }
        let fallback = self.tcc_fallback(&positions);

        // Dictionary candidates carry their LM context from the precomputed table (no per-edge
        // string hash); the TCC fallback edges (single clusters, not dict entries) resolve theirs
        // by lookup. Same edge/context ordering as the old `dict_candidates` + `contexts`.
        let (mut word_cands, mut word_ctx) = self.word_dict_candidates_ctx(text, &byte_to_idx);
        let word_model = self.bigram_model(text, &LatticeTier::Word);
        for fb in &fallback {
            word_ctx.push(word_model.layer.token_id.get(&text[fb.start..fb.end]).copied());
            word_cands.push(fb.clone());
        }
        let word_path = viterbi(&word_cands, &word_ctx, 0, byte_len, &word_model);

        // Coalesce OOV pieces into maximal spans for re-syllabification. A piece is OOV if it is a
        // TCC fallback OR a dictionary "word" that is a single bare consonant (`is_bare_consonant`):
        // letting those degenerate one-letter entries tile a transliterated run fragments it and
        // leaks a bare TCC into the output, so we fold them into the adjacent OOV span and let the
        // whole run be syllabified as one (บ|ลั|ช → บ|ลัช). An isolated single consonant simply
        // forms a length-1 OOV span and re-emerges as the same surface token (boundary-neutral).
        let mut spans: Vec<(usize, usize, bool)> = Vec::new();
        for &idx in &word_path {
            let c = &word_cands[idx];
            let is_oov = !matches!(c.payload, LatticeTier::Word)
                || is_bare_consonant(&text[c.start..c.end]);
            if is_oov {
                if let Some(last) = spans.last_mut()
                    && !last.2
                    && last.1 == c.start
                {
                    last.1 = c.end;
                    continue;
                }
                spans.push((c.start, c.end, false));
            } else {
                spans.push((c.start, c.end, true));
            }
        }

        // Baseline token list: dictionary words verbatim, OOV runs syllabified (syllable cands
        // and their precomputed contexts built lazily — only if some span actually needs them).
        let mut syl: Option<(Vec<Edge<LatticeTier>>, Vec<Option<u32>>)> = None;
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
                        &byte_to_idx,
                    );
                    c.extend(fallback.iter().cloned());
                    let ids = self.bigram_model(text, &LatticeTier::Syllable).contexts(&c);
                    (c, ids)
                });
                let syl_model = self.bigram_model(text, &LatticeTier::Syllable);
                for &i in &viterbi(&entry.0, &entry.1, s, e, &syl_model) {
                    let sy = &entry.0[i];
                    toks.push((sy.start, sy.end, sy.payload.clone()));
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
        let (positions, byte_to_idx) = self.tcc_grid(text);
        if positions.len() < 2 {
            return Vec::new();
        }

        let mut cands = self.dict_candidates(
            &self.syllable_trie,
            text,
            LatticeTier::Syllable,
            &byte_to_idx,
        );
        cands.extend(self.tcc_fallback(&positions));
        let model = self.bigram_model(text, &LatticeTier::Syllable);
        let ctx = model.contexts(&cands);

        viterbi(&cands, &ctx, 0, byte_len, &model)
            .iter()
            .map(|&i| pack(cands[i].start, cands[i].end, tier_flag(&cands[i].payload)))
            .collect()
    }
}
