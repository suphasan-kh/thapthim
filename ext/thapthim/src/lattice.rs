// ext/thapthim/src/lattice.rs

#[derive(Debug, Clone)]
pub struct LatticeEdge {
    pub start_tcc_idx: usize,
    pub end_tcc_idx: usize,
    pub text: String,
    pub granularity_weight: f64, // Higher bias for longer dictionary words
    pub probability: f64,        // Unigram/Bigram language model score
}

// 💡 This is a great place to attach helper methods to your struct later!
impl LatticeEdge {
    pub fn new(start: usize, end: usize, text: String, weight: f64, prob: f64) -> Self {
        Self {
            start_tcc_idx: start,
            end_tcc_idx: end,
            text,
            granularity_weight: weight,
            probability: prob,
        }
    }
}