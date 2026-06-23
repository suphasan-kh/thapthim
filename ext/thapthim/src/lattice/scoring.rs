// ext/thapthim/src/lattice/scoring.rs
//
// Kneser-Ney–smoothed bigram log-probability — the single scoring primitive the Viterbi decode
// calls on every lattice transition.
use super::*;

impl RuntimeEngine {
    /// Kneser-Ney–smoothed bigram log-probability under `lm_tier`'s layer. When `w1` is unseen
    /// by that layer (common for dictionary-only words), it backs off to `w2`'s continuation
    /// probability rather than a flat floor, so the score still reflects how connectable `w2` is.
    pub fn score_transition(&self, lm_tier: &LatticeTier, w1: &str, w2: &str) -> f64 {
        let layer = self.layer(lm_tier);
        let id1 = layer.token_id.get(w1).copied();
        let id2 = layer.token_id.get(w2).copied();
        let d = 0.75;
        let total_bigram_types = layer.total_bigram_types;

        // A token absent from the layer's vocab is exactly the old `count == 0` case.
        let count_w1 = id1.map_or(0, |i| layer.unigrams[i as usize].0);

        if count_w1 > 0 {
            let i1 = id1.unwrap();
            let unique_following_w1 = layer.followers[i1 as usize];
            let count_bi = id2
                .and_then(|i2| layer.bigrams.get(&((i1 as u64) << 32 | i2 as u64)))
                .copied()
                .unwrap_or(0);

            let lambda = (d / count_w1 as f64) * unique_following_w1 as f64;
            let (unigram_count, preceding_contexts) =
                id2.map_or((0, 0), |i| layer.unigrams[i as usize]);

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
            let preceding_contexts = id2.map_or(0, |i| layer.unigrams[i as usize].1) as f64;
            ((preceding_contexts + 1.0) / (total_bigram_types + 1.0)).ln() - self.oov_penalty
        }
    }
}
