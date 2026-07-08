// ext/thapthim/src/pos/mod.rs
//
// First-order (bigram) HMM part-of-speech tagger — a standalone decoder, NOT an instantiation of
// the segmentation grid (grid.rs). A POS lattice is dense and fixed-width: every token slot exposes
// all `NUM_TAGS` tags and every tag connects to all predecessors, so the natural representation is a
// flat `n × T` DP table with a tight `T×T` inner loop over a contiguous transition matrix — strictly
// less work than bucketing sparse edges. The engine layering mirrors the LM side:
//   * `PosModelFile` — the on-disk (bincode) parameters, minted offline by examples/train_pos_hmm.rs
//   * `HmmTagger`     — the runtime form, rehydrated from a `PosModelFile` (String maps → FxHashMap)
//   * `tag`           — log-space Viterbi decode over an already-segmented token sequence
//
// The tagger consumes WORDS, not raw text: run the segmenter first, then hand its tokens here. This
// keeps the two stages decoupled so a segmentation error never silently corrupts a POS number. The
// "accept a raw string" convenience is a binding-layer concern (Ruby: `word_segment` then `tag`), so
// this module never depends on the segmenter and eval can always feed gold tokens.
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The LST20 POS tagset — exactly 16 tags (see `datasets/LST20_full_train`, column 2). Order here
/// defines each tag's dense id (`Pos as usize`), which indexes every table below; keep it stable or
/// retrain. NB these are the POS tags (col 2), distinct from the NER (col 3) / clause (col 4) tags
/// in the same files.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Pos {
    NN, // noun
    VV, // verb
    PU, // punctuation (the space token `_` is tagged PU in LST20)
    CC, // conjunction
    PS, // preposition
    AX, // auxiliary
    AV, // adverb
    FX, // prefix/affix cluster
    NU, // number
    AJ, // adjective
    CL, // classifier
    PR, // pronoun
    NG, // negation
    PA, // particle
    XX, // unknown/foreign
    IJ, // interjection
}

/// Total tag count — the width of every DP row and the transition matrix side.
pub const NUM_TAGS: usize = 16;

impl Pos {
    /// Tags in dense-id order; `ALL[i].idx() == i`. Drives the Viterbi inner loop and training.
    pub const ALL: [Pos; NUM_TAGS] = [
        Pos::NN, Pos::VV, Pos::PU, Pos::CC, Pos::PS, Pos::AX, Pos::AV, Pos::FX,
        Pos::NU, Pos::AJ, Pos::CL, Pos::PR, Pos::NG, Pos::PA, Pos::XX, Pos::IJ,
    ];

    /// Dense id (0..NUM_TAGS) — the row/column index into every probability table.
    #[inline]
    pub fn idx(self) -> usize {
        self as usize
    }

    /// The LST20 surface string, e.g. `Pos::NN => "NN"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Pos::NN => "NN", Pos::VV => "VV", Pos::PU => "PU", Pos::CC => "CC",
            Pos::PS => "PS", Pos::AX => "AX", Pos::AV => "AV", Pos::FX => "FX",
            Pos::NU => "NU", Pos::AJ => "AJ", Pos::CL => "CL", Pos::PR => "PR",
            Pos::NG => "NG", Pos::PA => "PA", Pos::XX => "XX", Pos::IJ => "IJ",
        }
    }

    /// Parse an LST20 POS string; `None` for anything outside the 16-tag set (lets the trainer
    /// reject malformed lines instead of silently miscounting).
    pub fn from_str(s: &str) -> Option<Pos> {
        Pos::ALL.into_iter().find(|t| t.as_str() == s)
    }
}

/// Convention (1) surface canonicalization, shared by the trainer and the runtime so both agree on
/// what a token's emission key is. LST20 writes the space token as the literal `_` (tagged PU); the
/// real segmenter emits actual whitespace. We fold both — and any multi-space / newline run — to a
/// single ASCII space, so "space" is one vocabulary entry seen consistently at train and inference.
/// Everything else passes through untouched. Returns a borrow: the canonical space is a `'static`
/// literal, other tokens are returned as-is.
pub fn canon_surface(s: &str) -> &str {
    if s == "_" || (!s.is_empty() && s.chars().all(char::is_whitespace)) {
        " "
    } else {
        s
    }
}

/// True for a non-empty token whose every char is an Arabic (0-9) or Thai (๐-๙) digit. Such a token
/// is a number with near-certainty, so the OOV path hard-routes it to NU. Mixed tokens like `12:21`
/// or `3.5` fail this (they contain `:`/`.`) and fall through to the statistical UNK model instead.
fn is_all_digits(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| c.is_ascii_digit() || ('\u{0E50}'..='\u{0E59}').contains(&c))
}

/// True for a token that carries a letter but no Thai character — i.e. *any* non-Thai script. This is
/// script-agnostic on purpose: `char::is_alphabetic` is true for every Unicode letter (Latin, CJK,
/// Cyrillic, Arabic, Greek, Hangul, kana, Devanagari, Lao, …), so all of them are caught, not just
/// Latin/CJK. In LST20 such tokens are tagged NN essentially always (2650 NN : 7 NU among Latin
/// tokens, and the same foreign-noun convention holds across scripts), so the OOV path hard-routes
/// them to NN. Without this rule, foreign OOV words fall through to the statistical UNK model, where a
/// strong punctuation transition (e.g. `(` → NU, learned from parenthesized years/citations) drags
/// them to NU — the abnormality this fixes. Pure-symbol tokens (no letter, e.g. emoji) are not
/// foreign and stay on the UNK path. NB this only sees *script*: a foreign name transliterated into
/// Thai script has Thai chars, so it is (correctly) not caught here — that case is not orthographic
/// (see [[thapthim-char-statistical-translit]]).
fn is_foreign(s: &str) -> bool {
    let mut has_letter = false;
    for c in s.chars() {
        if ('\u{0E00}'..='\u{0E7F}').contains(&c) {
            return false; // any Thai char → not a foreign token
        }
        has_letter |= c.is_alphabetic();
    }
    has_letter
}

/// True if the token carries the Thai thanthakhat (karan) mark U+0E4C — the silencer over a final
/// consonant. Such tokens are overwhelmingly nouns: 96% NN among rare/OOV LST20 tokens (and 21% of
/// *all* rare tokens carry it), because karan marks loanwords, transliterations, and formal/technical
/// terms. Crucially this catches the residual OOV failure `is_foreign` cannot — foreign *names*
/// transliterated into Thai script (อินโนเซ้นท์, แกรนด์สปอร์ต), which are Thai script yet carry karan —
/// so the OOV path routes karan tokens to NN alongside foreign script.
fn has_karan(s: &str) -> bool {
    s.contains('\u{0E4C}')
}

/// True for a non-empty token with no letter and no digit — pure punctuation/symbols (100% PU in
/// LST20). An OOV symbol run routes to PU. (`is_all_digits` is checked first, so digits don't reach
/// here; `is_alphanumeric` covers Thai/Latin/… letters and digits alike.)
fn is_all_symbol(s: &str) -> bool {
    !s.is_empty() && !s.chars().any(char::is_alphanumeric)
}

/// Productive Thai noun morphology on an OOV token: the derivational prefixes การ / ความ (nominalizers)
/// and ผู้ (agentive), and the suffix ศาสตร์ ("-ology"), all form nouns — so a longer OOV word carrying
/// one is reliably NN (94–100% on rare LST20 tokens) and the OOV path routes it to NN. Length-guarded
/// so the bare affix word itself is not matched (it is in-vocab and never reaches the OOV path anyway).
/// Byte lengths compare directly: every Thai code point is 3 UTF-8 bytes, so `starts_with`/`ends_with`
/// plus a `len()` guard is exact. (ภาพ was measured too but left out — 86% is below the shape-rule bar.)
fn has_noun_affix(s: &str) -> bool {
    const PREFIXES: [&str; 3] = ["การ", "ความ", "ผู้"];
    const SUFFIXES: [&str; 1] = ["ศาสตร์"];
    PREFIXES.iter().any(|p| s.len() > p.len() && s.starts_with(p))
        || SUFFIXES.iter().any(|p| s.len() > p.len() && s.ends_with(p))
}

/// Log-prob handed to every tag except the shape-selected one for an OOV token matching a shape rule
/// (NU for digits, PU for symbols, NN for foreign script / karan): large negative so that tag wins the
/// column outright, but finite so a bizarre transition context can still, in principle, override it.
const OOV_SHAPE_REJECT: f64 = -30.0;

/// Serialized HMM parameters — what the offline trainer writes and the runtime loads (bincode, like
/// the interned LM). All probabilities are natural-log so the decode is additive. Uses std `HashMap`
/// for portable (de)serialization; `HmmTagger` swaps in `FxHashMap` at load, mirroring how
/// `RuntimeLayer::from_interned` rehydrates the LM.
#[derive(Serialize, Deserialize)]
pub struct PosModelFile {
    /// log P(tag | BOS) — the sentence-initial tag distribution. Length `NUM_TAGS`.
    pub log_init: Vec<f32>,
    /// log P(cur | prev), row-major `prev*NUM_TAGS + cur`. Length `NUM_TAGS * NUM_TAGS`.
    pub log_trans: Vec<f32>,
    /// Surface word → dense word id. Keys are already `canon_surface`-normalized. OOV words (absent
    /// here) route through the Witten-Bell UNK path below.
    pub vocab: HashMap<String, u32>,
    /// Per-tag emission for seen words: `log_emit[tag][word_id] = log P(word | tag)`. Sparse — most
    /// words take a handful of tags. Length `NUM_TAGS`.
    pub log_emit: Vec<HashMap<u32, f32>>,
    /// Per-tag unknown-word emission: `log_unk_emit[tag] = log P(UNK | tag)`, the Witten-Bell mass a
    /// tag reserves for unseen words (estimated from its distinct-type count at train time). Length
    /// `NUM_TAGS`. Open classes (NN, VV) reserve far more than closed ones (NG, CL), which is exactly
    /// the tag-discriminative OOV signal a uniform fallback throws away.
    pub log_unk_emit: Vec<f32>,
}

/// Runtime tagger: the rehydrated parameters plus the decode. Build once (it owns ~a few MB of
/// tables) and reuse across calls, like `RuntimeEngine`.
pub struct HmmTagger {
    log_init: Vec<f32>,
    log_trans: Vec<f32>,
    vocab: FxHashMap<String, u32>,
    log_emit: Vec<FxHashMap<u32, f32>>,
    log_unk_emit: Vec<f32>,
}

/// A token resolved to its emission source once (hoisted out of the per-tag inner loop): either a
/// vocab id (seen word) or the canonicalized surface (OOV, scored by shape + UNK model). Resolving
/// this once per token turns the decode's per-token cost from 16 string-hashes into one.
enum Resolved<'a> {
    Known(u32),
    Oov(&'a str),
}

impl HmmTagger {
    /// Rehydrate from a deserialized `PosModelFile` (String maps → FxHashMap).
    pub fn from_file(m: PosModelFile) -> Self {
        let vocab = m.vocab.into_iter().collect();
        let log_emit = m
            .log_emit
            .into_iter()
            .map(|tbl| tbl.into_iter().collect())
            .collect();
        HmmTagger {
            log_init: m.log_init,
            log_trans: m.log_trans,
            vocab,
            log_emit,
            log_unk_emit: m.log_unk_emit,
        }
    }

    /// Rehydrate from an in-memory bincode model — the embedded-asset path (`include_bytes!`), which
    /// never touches the filesystem. `load_from_path` delegates here after reading the file.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let file: PosModelFile =
            bincode::deserialize(bytes).expect("deserialize pos_hmm model (format mismatch)");
        Self::from_file(file)
    }

    /// Load a bincode model from `path` — the `THAPTHIM_POS_MODEL` override used for experiments and
    /// by the trainer/eval. The shipped default is the embedded asset (see `crate::get_pos_tagger`).
    pub fn load_from_path(path: &str) -> Self {
        let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("pos model {path}: {e}"));
        Self::from_bytes(&bytes)
    }

    /// Whether `word` (after canonicalization) was in the training vocabulary — i.e. it is scored by
    /// the trained emission table rather than the OOV path. Exposed so eval can split accuracy into
    /// known vs OOV without re-deriving the vocab.
    pub fn is_known(&self, word: &str) -> bool {
        self.vocab.contains_key(canon_surface(word))
    }

    /// Public emission accessor for experiments/tools (e.g. a trigram A/B): `log P(word | tag)` using
    /// the exact known-word lookup + OOV shape/UNK fallback the decoder uses, so a harness can vary
    /// only the transition model while holding emission fixed.
    pub fn emission_logprob(&self, word: &str, tag: usize) -> f64 {
        self.emission(&self.resolve(word), tag)
    }

    /// log P(cur | prev) — a flat transition-matrix lookup.
    #[inline]
    fn transition(&self, prev: usize, cur: usize) -> f64 {
        self.log_trans[prev * NUM_TAGS + cur] as f64
    }

    /// Resolve a raw token to its emission source once: canonicalize the surface (convention 1),
    /// then classify as a known vocab id or an OOV surface.
    #[inline]
    fn resolve<'a>(&self, word: &'a str) -> Resolved<'a> {
        let w = canon_surface(word);
        match self.vocab.get(w) {
            Some(&wid) => Resolved::Known(wid),
            None => Resolved::Oov(w),
        }
    }

    /// log P(word | tag) for a pre-resolved token.
    #[inline]
    fn emission(&self, tok: &Resolved, tag: usize) -> f64 {
        match *tok {
            // Seen word, but maybe never with THIS tag → NEG_INFINITY: the generative model forbids
            // a tag it never emitted this word under. This constrains the tag space tightly (usually
            // a precision win); loosen it here if a frequent word ever needs an unseen tag at test.
            Resolved::Known(wid) => {
                self.log_emit[tag].get(&wid).map(|&lp| lp as f64).unwrap_or(f64::NEG_INFINITY)
            }
            Resolved::Oov(w) => self.emission_oov(w, tag),
        }
    }

    /// OOV emission — the HMM's weak spot, worst cross-domain (per [[thapthim-loo-no-transfer]] the
    /// OOV rate balloons off-domain even though it's only ~1.7% on the in-domain LST20 test split).
    /// Layered, cheapest first:
    ///   1. **Shape rules** — high-precision orthographic routing that overrides transition context
    ///      (each returns `0.0` for the selected tag, `OOV_SHAPE_REJECT` for the rest); precision and
    ///      share measured on rare/OOV LST20 tokens:
    ///        - all-digit token            → **NU**  (`is_all_digits`)
    ///        - all-symbol (no letter/digit) → **PU**  (`is_all_symbol`; 100%)
    ///        - any non-Thai script token  → **NN**  (`is_foreign`; the LST20 foreign-noun convention)
    ///        - Thai karan mark ◌์ present → **NN**  (`has_karan`; 96%, 21% of rare tokens) — recovers
    ///          transliterated foreign *names* in Thai script that `is_foreign` misses
    ///        - Thai noun morphology (การ/ความ/ผู้ prefix, ศาสตร์ suffix) → **NN**  (`has_noun_affix`; 94–100%)
    ///      These exist because a bare Witten-Bell emission is weak enough that punctuation transitions
    ///      hijack it — parenthesized-number context (`(` → NU) otherwise mis-tags foreign words NU.
    ///   2. **Witten-Bell UNK emission** — the statistical fallback for everything else (native Thai
    ///      OOV), tag-discriminative: open classes reserve more unseen mass than closed ones.
    /// The residual (un-marked transliterations, no karan) stays hard: foreignness there is in the
    /// syllable sequence, not the characters ([[thapthim-char-statistical-translit]]); a trained
    /// suffix→tag table is the natural next lever.
    #[inline]
    fn emission_oov(&self, word: &str, tag: usize) -> f64 {
        if is_all_digits(word) {
            return if tag == Pos::NU.idx() { 0.0 } else { OOV_SHAPE_REJECT };
        }
        if is_all_symbol(word) {
            return if tag == Pos::PU.idx() { 0.0 } else { OOV_SHAPE_REJECT };
        }
        if is_foreign(word) || has_karan(word) || has_noun_affix(word) {
            return if tag == Pos::NN.idx() { 0.0 } else { OOV_SHAPE_REJECT };
        }
        self.log_unk_emit[tag] as f64
    }

    /// Viterbi decode: the single most probable tag sequence for `tokens` (already segmented words).
    /// Log-space, so scores add. Returns one `Pos` per input token; empty in → empty out.
    ///
    /// Generic over `AsRef<str>` so it takes both `&[&str]` (tests / gold-token eval) and
    /// `&[String]` (the Ruby FFI decode and the string-in cascade) without a copy. This is the single
    /// pure seam; the raw-string convenience lives in the binding layer, not here.
    pub fn tag<S: AsRef<str>>(&self, tokens: &[S]) -> Vec<Pos> {
        let n = tokens.len();
        if n == 0 {
            return Vec::new();
        }
        let t = NUM_TAGS;

        // Hoist the vocab lookup out of the T×T DP: resolve each token's emission source once.
        let resolved: Vec<Resolved> = tokens.iter().map(|s| self.resolve(s.as_ref())).collect();

        // dp[i*T + s] = best log-prob of a tag path over tokens[0..=i] ending in tag s.
        // bp[i*T + s] = the tag at i-1 on that best path (for backtracking).
        let mut dp = vec![f64::NEG_INFINITY; n * t];
        let mut bp = vec![0u8; n * t];

        // Initial column: BOS→tag transition (log_init) + emission.
        for s in 0..t {
            dp[s] = self.log_init[s] as f64 + self.emission(&resolved[0], s);
        }

        // Forward pass.
        for i in 1..n {
            for s in 0..t {
                let em = self.emission(&resolved[i], s);
                if em == f64::NEG_INFINITY {
                    continue; // tag impossible for this seen word; leave NEG_INFINITY
                }
                let mut best = f64::NEG_INFINITY;
                let mut arg = 0usize;
                for p in 0..t {
                    let prev = dp[(i - 1) * t + p];
                    if prev == f64::NEG_INFINITY {
                        continue;
                    }
                    let score = prev + self.transition(p, s);
                    if score > best {
                        best = score;
                        arg = p;
                    }
                }
                if best != f64::NEG_INFINITY {
                    dp[i * t + s] = best + em;
                    bp[i * t + s] = arg as u8;
                }
            }
        }

        // Terminate: best tag in the last column, then walk backpointers.
        let last = (n - 1) * t;
        let mut cur = (0..t).max_by(|&a, &b| dp[last + a].total_cmp(&dp[last + b])).unwrap();
        let mut out = vec![Pos::NN; n]; // overwritten below; NN is an arbitrary placeholder
        for i in (0..n).rev() {
            out[i] = Pos::ALL[cur];
            if i > 0 {
                cur = bp[i * t + cur] as usize;
            }
        }
        out
    }
}
