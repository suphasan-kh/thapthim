// ext/thapthim/src/lattice/entropy.rs
//
// Branching-entropy OOV-merge post-pass: dissolves boundaries inside a run of short pieces that
// actually tile one unknown word (ลัน|ดัน → ลันดัน). Driven by the embedded char-entropy tables.
use super::*;
use rustc_hash::FxHashMap;

impl RuntimeEngine {
    /// Longest-suffix lookup: try the full slice, then drop leading chars, returning the entropy
    /// of the longest context present in `map` (the most specific evidence available).
    fn lookup_suffix(map: &FxHashMap<String, f32>, chars: &[char]) -> Option<f64> {
        for l in (1..=chars.len()).rev() {
            let key: String = chars[chars.len() - l..].iter().collect();
            if let Some(&h) = map.get(&key) {
                return Some(h as f64);
            }
        }
        None
    }

    /// Longest-prefix lookup: mirror of `lookup_suffix` for the right-context (backward) table.
    fn lookup_prefix(map: &FxHashMap<String, f32>, chars: &[char]) -> Option<f64> {
        for l in (1..=chars.len()).rev() {
            let key: String = chars[..l].iter().collect();
            if let Some(&h) = map.get(&key) {
                return Some(h as f64);
            }
        }
        None
    }

    /// Averaged branching entropy at byte boundary `p` inside `text`: the right-branching entropy
    /// of the up-to-K chars ending at `p` and the left-branching entropy of the up-to-K chars
    /// starting at `p`. Low → the two sides cohere (word-internal); high → a real boundary.
    /// `None` when neither side is attested, in which case the caller keeps the boundary (safe).
    fn oov_boundary_entropy(&self, text: &str, p: usize) -> Option<f64> {
        let mut left: Vec<char> = text[..p].chars().rev().take(BE_MAX_CTX).collect();
        left.reverse();
        let right: Vec<char> = text[p..].chars().take(BE_MAX_CTX).collect();

        let h_fwd = Self::lookup_suffix(&self.char_entropy_fwd, &left);
        let h_bwd = Self::lookup_prefix(&self.char_entropy_bwd, &right);
        match (h_fwd, h_bwd) {
            (Some(a), Some(b)) => Some((a + b) / 2.0),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    }

    /// Branching-entropy merge over the final word-path tokens. A boundary between two tokens is
    /// dissolved only when BOTH neighbouring tokens are short (≤ `BE_MERGE_MAX_TCC` clusters) AND
    /// the averaged branching entropy there is below `be_threshold` — i.e. a run of short pieces
    /// that cohere into one unknown word (the ลัน|ดัน / transliteration over-split, where the
    /// pieces are often *short dictionary words*, not just OOV fallbacks). Gating on length keeps
    /// ordinary in-vocab segmentation (longer content words) untouched, so modern-text accuracy is
    /// structurally preserved; the entropy threshold guards each individual merge. Glued runs are
    /// retagged `Word` (they now denote one word); singletons keep their original tier.
    pub(super) fn merge_short_runs(
        &self,
        text: &str,
        toks: &[(usize, usize, LatticeTier)],
        byte_to_idx: &[u32],
    ) -> Vec<(usize, usize, LatticeTier)> {
        // Token endpoints are always grid boundaries, so these indices are never `NON_BOUNDARY`.
        let short = |s: usize, e: usize| (byte_to_idx[e] - byte_to_idx[s]) as usize <= self.be_max_tcc;
        let mut out = Vec::with_capacity(toks.len());
        let n = toks.len();
        let mut i = 0;
        while i < n {
            let start = toks[i].0;
            let mut end = toks[i].1;
            let mut tier = toks[i].2.clone();
            let mut j = i;
            while j + 1 < n {
                // Gate on the ORIGINAL neighbouring pieces (not the growing left side), so a whole
                // run of short pieces can fuse into one long word while a long token still blocks.
                if !(short(toks[j].0, toks[j].1) && short(toks[j + 1].0, toks[j + 1].1)) {
                    break;
                }
                match self.oov_boundary_entropy(text, toks[j].1) {
                    Some(h) if h < self.be_threshold => {
                        j += 1;
                        end = toks[j].1;
                        tier = LatticeTier::Word;
                    }
                    _ => break,
                }
            }
            out.push((start, end, tier));
            i = j + 1;
        }
        out
    }
}
