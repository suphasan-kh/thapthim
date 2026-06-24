// ext/thapthim/src/lattice/scoring.rs
//
// Kneser-Ney–smoothed bigram log-probability — the single scoring primitive the Viterbi decode
// calls on every lattice transition.
use super::*;

impl RuntimeLayer {
    /// Kneser-Ney–smoothed bigram log-probability from PRECOMPUTED token ids — the decode hot
    /// path. Taking ids (resolved once per lattice node) instead of `&str` keeps the per-edge
    /// inner loop free of String hashing and key comparison. When `id1` is unseen by this layer
    /// (common for dictionary-only words), it backs off to `id2`'s continuation probability rather
    /// than a flat floor, so the score still reflects how connectable the follower is.
    ///
    /// May return `f64::NEG_INFINITY` (a seen `id1` with no followers and a zero bigram count
    /// makes the log argument 0), so callers must track node reachability separately rather than
    /// treating a NEG_INFINITY score as "unreached".
    #[inline]
    pub(super) fn score(&self, id1: Option<u32>, id2: Option<u32>, oov_penalty: f64) -> f64 {
        let d = 0.75;
        let total_bigram_types = self.total_bigram_types;

        // A token absent from the layer's vocab is exactly the old `count == 0` case.
        let count_w1 = id1.map_or(0, |i| self.unigrams[i as usize].0);

        if count_w1 > 0 {
            let i1 = id1.unwrap();
            let unique_following_w1 = self.followers[i1 as usize];
            let count_bi = id2
                .and_then(|i2| self.bigrams.get(&((i1 as u64) << 32 | i2 as u64)))
                .copied()
                .unwrap_or(0);

            let lambda = (d / count_w1 as f64) * unique_following_w1 as f64;
            let (unigram_count, preceding_contexts) =
                id2.map_or((0, 0), |i| self.unigrams[i as usize]);

            let p_continuation = if preceding_contexts > 0 {
                preceding_contexts as f64 / total_bigram_types
            } else {
                (unigram_count.max(1) as f64) / total_bigram_types
            };

            let primary_term = ((count_bi as f64 - d).max(0.0)) / count_w1 as f64;
            (primary_term + lambda * p_continuation).ln()
        } else {
            // w1 unseen by this layer: back off to w2's (add-one smoothed) continuation prob,
            // minus a penalty so an unseen (often dictionary-only) context is trusted less than a
            // seen one — stops junk dict words like กอดอก beating an attested decomposition.
            let preceding_contexts = id2.map_or(0, |i| self.unigrams[i as usize].1) as f64;
            ((preceding_contexts + 1.0) / (total_bigram_types + 1.0)).ln() - oov_penalty
        }
    }
}

impl RuntimeEngine {
    /// String-keyed convenience wrapper: resolve `w1`/`w2` to ids, then `RuntimeLayer::score`.
    /// The decode hot path precomputes ids per node and calls `RuntimeLayer::score` directly to
    /// avoid re-hashing the same token string on every incident edge.
    pub fn score_transition(&self, lm_tier: &LatticeTier, w1: &str, w2: &str) -> f64 {
        let layer = self.layer(lm_tier);
        layer.score(layer.token_id.get(w1).copied(), layer.token_id.get(w2).copied(), self.oov_penalty)
    }
}
