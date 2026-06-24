// ext/thapthim/src/lattice/mod.rs
//
// The grid-aligned bigram-Viterbi segmentation engine, split by function group:
//   mod.rs    — types, tunables, packing helpers, and engine construction (this file)
//   scoring   — Kneser-Ney bigram log-probability (`score_transition`)
//   decode    — TCC grid, candidate generation, Viterbi, and the `segment_*` entry points
//   entropy   — branching-entropy OOV-merge post-pass
// All four share one `RuntimeEngine`; the child modules add `impl RuntimeEngine` blocks and
// reach its private fields through Rust's ancestor-module visibility.
use rustc_hash::FxHashMap;
use daachorse::CharwiseDoubleArrayAhoCorasick;
use crate::lm_format::{InternedLayer, InternedModel};
use crate::tcc::TccSegmenter;

mod scoring;
mod decode;
mod entropy;

/// Interned runtime form of one LM layer. Tokens are looked up to dense ids via `token_id`; all
/// count tables are then id-keyed. `followers[id]` is the Kneser-Ney `N₁₊(w1 •)` term (distinct
/// types after `w1`), precomputed at load so the Viterbi hot path never scans the bigram table.
struct RuntimeLayer {
    token_id: FxHashMap<String, u32>, // token -> id (query-time); the only String store that remains
    unigrams: Vec<(u32, u32)>,        // id -> (count, preceding_contexts)
    bigrams: FxHashMap<u64, u32>,     // (w1_id << 32 | w2_id) -> count
    followers: Vec<u32>,              // id -> N₁₊(w1 •)
    total_bigram_types: f64,          // |bigrams|, hoisted out of the hot path
}

impl RuntimeLayer {
    /// Rehydrate an interned layer: build the token→id map, move the dense unigram table, rebuild
    /// the bigram hashmap from the packed Vec, and tally follower types in the same pass.
    fn from_interned(layer: InternedLayer) -> Self {
        let n = layer.vocab.len();
        let mut token_id = FxHashMap::default();
        token_id.reserve(n);
        for (i, tok) in layer.vocab.into_iter().enumerate() {
            token_id.insert(tok, i as u32);
        }

        let mut followers = vec![0u32; n];
        let mut bigrams = FxHashMap::default();
        bigrams.reserve(layer.bigrams.len());
        for (key, count) in layer.bigrams {
            followers[(key >> 32) as usize] += 1; // each packed key is unique -> N₁₊(w1 •)
            bigrams.insert(key, count);
        }

        let total_bigram_types = bigrams.len() as f64;
        RuntimeLayer { token_id, unigrams: layer.unigrams, bigrams, followers, total_bigram_types }
    }
}

pub struct RuntimeEngine {
    pub word_trie: CharwiseDoubleArrayAhoCorasick<usize>,
    pub syllable_trie: CharwiseDoubleArrayAhoCorasick<usize>,
    /// The TCC grid segmenter, compiled once at bootstrap. Its master regex is expensive to
    /// build, so it must be reused across calls rather than reconstructed per `segment`.
    tcc: TccSegmenter,
    words: RuntimeLayer,
    syllables: RuntimeLayer,
    tccs: RuntimeLayer,
    /// Character branching-entropy tables (built offline from tnhc_train by
    /// tools/build_char_entropy.rb). `fwd` keys a left-context to the right-branching entropy of
    /// the next char; `bwd` keys a right-context to the left-branching entropy of the prev char.
    /// Used only to merge over-segmented OOV runs — see `oov_boundary_entropy`.
    char_entropy_fwd: FxHashMap<String, f32>,
    char_entropy_bwd: FxHashMap<String, f32>,
    /// OOV-merge aggressiveness: an internal OOV boundary whose averaged branching entropy is
    /// below this is treated as word-internal and dissolved. `<= 0` disables the pass entirely
    /// (identical to the pre-entropy behavior). Set via `THAPTHIM_BE_THRESHOLD` at bootstrap.
    be_threshold: f64,
    /// Max TCC length of a token that may participate in a merge (the length gate). Tunable via
    /// `THAPTHIM_BE_MAX_TCC` for sweeps; defaults to `BE_MERGE_MAX_TCC`.
    be_max_tcc: usize,
    /// Log-prob penalty subtracted from a transition *out of* a word unseen by the LM (the OOV
    /// backoff branch). Larger = trust dictionary-only/unseen words less, so an attested
    /// decomposition wins over a junk dict entry like `กอดอก`. Set via `THAPTHIM_OOV_PENALTY`.
    oov_penalty: f64,
    /// Max length (in TCC clusters, NOT characters) of a dictionary word candidate; longer matches
    /// are dropped from the lattice. Set via `THAPTHIM_MAX_WORD_TCC` (`0` = no cap).
    max_word_tcc: usize,
}

/// Longest context fed to the branching-entropy tables. Must match K in build_char_entropy.rb.
const BE_MAX_CTX: usize = 4;

/// Default OOV-merge threshold when `THAPTHIM_BE_THRESHOLD` is unset. Tuned on tnhc_train (held-out
/// tnhc_test: F1 0.7742→0.7760, precision +0.0067; LST20 guard −0.0005). `0.0` disables the pass.
const DEFAULT_BE_THRESHOLD: f64 = 1.0;

/// Only boundaries between tokens of at most this many TCC clusters are merge candidates. Short
/// runs are the over-split symptom (e.g. a 2-cluster transliteration piece like ลัน); longer
/// content words are left alone so in-vocab segmentation is preserved. Tuned with the threshold.
const BE_MERGE_MAX_TCC: usize = 2;

/// Default penalty (nats) on the unseen-w1 OOV backoff. Tuned to 2.0 — VISTEC-optimal and improves
/// every corpus over the old un-penalized scoring (LST20 +0.005, TNHC +0.015, VISTEC +0.0004) while
/// fixing junk-dict-word over-merges like กอดอก→กอ|ดอก|ไม้. Higher helps LST20/TNHC more but
/// over-segments VISTEC's heavier OOV tail. `0.0` restores legacy scoring.
const DEFAULT_OOV_PENALTY: f64 = 2.0;

/// Default max length (in TCC clusters) of a dictionary word candidate. Tuned by sweep: LST20 F1
/// is a flat plateau over 10–12 TCC (peak 10 = 0.9480 vs 12 = 0.9476, noise) and collapses below 8
/// (cap 6 → 0.921); BEST rises monotonically with the cap. 12 is kept as the best all-rounder in
/// the LST20-safe range. NB the reference paper's 80th-pct word length is a CHARACTER figure — a
/// literal 12-char cap (≈6 TCC) would crater LST20. `THAPTHIM_MAX_WORD_TCC` overrides; `0` = no cap.
const DEFAULT_MAX_WORD_TCC: usize = 12;

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
        let words_raw = include_str!("../../assets/master_words_vocab.txt");
        let mut word_patterns = Vec::new();
        for (idx, line) in words_raw.lines().enumerate() {
            if !line.is_empty() {
                word_patterns.push((line.to_string(), idx));
            }
        }
        let word_trie = CharwiseDoubleArrayAhoCorasick::with_values(word_patterns).unwrap();

        let syllables_raw = include_str!("../../assets/master_syllables_vocab.txt");
        let mut syllable_patterns = Vec::new();
        for (idx, line) in syllables_raw.lines().enumerate() {
            if !line.is_empty() {
                syllable_patterns.push((line.to_string(), idx));
            }
        }
        let syllable_trie = CharwiseDoubleArrayAhoCorasick::with_values(syllable_patterns).unwrap();

        // The interned LM is derived from the corpus pipeline's joint_lm.bin by build.rs and
        // embedded here (see src/lm_format.rs for the format and the lossless re-encode). The
        // default LST20-trained LM always ships; the alternate BEST-trained LM is embedded ONLY
        // under the `best_lm` cargo feature (off by default — see Cargo.toml), and selected at
        // bootstrap with THAPTHIM_LM=best. The dictionary/entropy assets are identical either way,
        // so this isolates the effect of the LM's training corpus. Without the feature, THAPTHIM_LM
        // is ignored and the default LM is always used.
        let lm_bytes: &[u8] = match std::env::var("THAPTHIM_LM").ok().as_deref() {
            #[cfg(feature = "best_lm")]
            Some("best") => &include_bytes!("../../assets/joint_lm_interned_best.bin")[..],
            _ => &include_bytes!("../../assets/joint_lm_interned.bin")[..],
        };
        let model: InternedModel = bincode::deserialize(lm_bytes)
            .expect("Critical error reading interned LM. Structure formats mismatch.");
        let words = RuntimeLayer::from_interned(model.words);
        let syllables = RuntimeLayer::from_interned(model.syllables);
        let tccs = RuntimeLayer::from_interned(model.tccs);

        let (char_entropy_fwd, char_entropy_bwd) = Self::load_char_entropy();
        let be_threshold = std::env::var("THAPTHIM_BE_THRESHOLD")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(DEFAULT_BE_THRESHOLD);
        let be_max_tcc = std::env::var("THAPTHIM_BE_MAX_TCC")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(BE_MERGE_MAX_TCC);
        let oov_penalty = std::env::var("THAPTHIM_OOV_PENALTY")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(DEFAULT_OOV_PENALTY);
        let max_word_tcc = std::env::var("THAPTHIM_MAX_WORD_TCC")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_WORD_TCC);

        RuntimeEngine {
            word_trie,
            syllable_trie,
            tcc: TccSegmenter::new(),
            words,
            syllables,
            tccs,
            char_entropy_fwd,
            char_entropy_bwd,
            be_threshold,
            be_max_tcc,
            oov_penalty,
            max_word_tcc,
        }
    }

    /// Parse the embedded branching-entropy table into its forward/backward maps. Each line is
    /// `dir<TAB>context<TAB>entropy` with dir `F` (forward) or `B` (backward).
    fn load_char_entropy() -> (FxHashMap<String, f32>, FxHashMap<String, f32>) {
        let raw = include_str!("../../assets/char_entropy.txt");
        let mut fwd = FxHashMap::default();
        let mut bwd = FxHashMap::default();
        for line in raw.lines() {
            let mut it = line.splitn(3, '\t');
            if let (Some(dir), Some(ctx), Some(ent)) = (it.next(), it.next(), it.next())
                && let Ok(h) = ent.parse::<f32>() {
                    match dir {
                        "F" => { fwd.insert(ctx.to_string(), h); }
                        "B" => { bwd.insert(ctx.to_string(), h); }
                        _ => {}
                    }
                }
        }
        (fwd, bwd)
    }

    fn layer(&self, lm_tier: &LatticeTier) -> &RuntimeLayer {
        match lm_tier {
            LatticeTier::Word => &self.words,
            LatticeTier::Syllable => &self.syllables,
            LatticeTier::Tcc => &self.tccs,
        }
    }
}
