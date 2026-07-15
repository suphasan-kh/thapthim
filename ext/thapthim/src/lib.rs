// ext/thapthim/src/lib.rs
pub mod tcc;
pub mod lattice;
// Shared with build.rs (which uses the String-keyed types + intern_*); the lib only reads the
// interned types, so the rest is "dead" from the lib's view — silence those warnings. `pub` so the
// `build_interned_lm` example can reach build_lm_from_kn/intern_model to mint an alternate LM asset.
#[allow(dead_code)]
pub mod lm_format;
// Thai text normalization, shared by the Ruby and Python `normalize:` options (ported from
// lib/thapthim/normalize_std.rb). Always built — both the C FFI and the PyO3 layer call it.
pub mod normalize;

// First-order HMM part-of-speech tagger (standalone dense decode; consumes segmented tokens). `pub`
// so examples/train_pos_hmm.rs can reach the tagset + model types. FFI wiring is a later step — see
// the module header. Some fields are dead until the runtime constructor lands; silence for now.
#[allow(dead_code)]
pub mod pos;

// Word-level lexical spelling correction (Norvig / noisy-channel over the thai_words dictionary +
// unigram LM). Built on top of the normalize layer; the sentence-level lattice variant comes later.
pub mod spell;

// Python (PyO3) binding — same engine, different outer layer. Gated so the default Ruby build is
// untouched and rb-sys/pyo3 never coexist in one compile.
#[cfg(feature = "python")]
mod python;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::OnceLock;
use crate::tcc::TccSegmenter;
use crate::lattice::RuntimeEngine;
use crate::pos::HmmTagger;

// Thread-safe global single allocation instance lock for the 40MB engine matrix
static ENGINE: OnceLock<RuntimeEngine> = OnceLock::new();

pub(crate) fn get_engine() -> &'static RuntimeEngine {
    ENGINE.get_or_init(|| {
        RuntimeEngine::bootstrap()
    })
}

// Process-global TccSegmenter. `TccSegmenter::new()` compiles a ~30-branch regex, so building one
// per call (as the standalone TCC entry points used to) dominated their cost. Cache it once;
// `find_*positions` take `&self` and the regex is read-only, so it's safe to share across threads
// and callers. Shared by the C FFI (Ruby) and the PyO3 layer.
static TCC_SEGMENTER: OnceLock<TccSegmenter> = OnceLock::new();

pub(crate) fn get_tcc() -> &'static TccSegmenter {
    TCC_SEGMENTER.get_or_init(TccSegmenter::new)
}

// Process-global POS tagger. The shipped model (assets/pos_hmm.bin, a bincode HMM minted by
// `cargo run --example train_pos_hmm` from LST20) is embedded in the binary and loaded with no
// filesystem access — so `pos_tag` works out of the box, exactly like segmentation's embedded LM.
// `THAPTHIM_POS_MODEL=<path>` overrides it with an external model for experiments (mirrors
// `THAPTHIM_LM`); the trainer and eval use that override to point at a work-in-progress model.
static POS_TAGGER: OnceLock<HmmTagger> = OnceLock::new();

pub(crate) fn get_pos_tagger() -> &'static HmmTagger {
    POS_TAGGER.get_or_init(|| match std::env::var("THAPTHIM_POS_MODEL") {
        Ok(path) => HmmTagger::load_from_path(&path),
        Err(_) => HmmTagger::from_bytes(include_bytes!("../assets/pos_hmm.bin")),
    })
}

// Process-global spelling engine (dictionary + unigram frequencies, ~a few MB). Built once.
static SPELL: OnceLock<crate::spell::SpellEngine> = OnceLock::new();

pub(crate) fn get_spell() -> &'static crate::spell::SpellEngine {
    SPELL.get_or_init(crate::spell::SpellEngine::bootstrap)
}

/// Reads a (non-null) C string pointer as UTF-8, replacing any invalid bytes with
/// U+FFFD instead of discarding the entire input. The Ruby layer already hands us
/// clean UTF-8, so this is defense-in-depth for direct C-ABI callers (e.g. future
/// language bindings) — a single bad byte should never blank out the whole result.
unsafe fn read_utf8<'a>(ptr: *const c_char) -> std::borrow::Cow<'a, str> {
    String::from_utf8_lossy(unsafe { CStr::from_ptr(ptr) }.to_bytes())
}

/// True when the packed token's surface is entirely whitespace (Unicode `White_Space`). Both
/// language bindings use this to implement their `keep_whitespace: false` default: the engine
/// itself always tiles the full input (losslessness is an engine invariant — whitespace runs come
/// back as tokens like any other span); dropping them is presentation, done here at the boundary
/// so Ruby and Python agree exactly on what counts as whitespace.
pub(crate) fn is_whitespace_token(text: &str, tok: u64) -> bool {
    let start = (tok >> 32) as usize;
    let len = ((tok >> 8) & 0xFF_FFFF) as usize;
    text.as_bytes()
        .get(start..start + len)
        .is_some_and(|b| !b.is_empty() && String::from_utf8_lossy(b).chars().all(char::is_whitespace))
}

/// Leaks `v` into a raw `(pointer, count)` pair for the C boundary. Returns null/0 instead when
/// the count would not fit the `i32` out-parameter: `len() as i32` on a >2GB result would wrap
/// negative, and that poisoned count later reaches the matching `thapthim_free_*` where it would
/// reconstruct the slice with a wildly wrong length (undefined behavior). Refusing the result is
/// the only safe answer the current ABI can give.
fn into_ffi_buffer<T>(v: Vec<T>) -> (*const T, i32) {
    if v.len() > i32::MAX as usize {
        return (std::ptr::null(), 0);
    }
    let len = v.len() as i32;
    (Box::into_raw(v.into_boxed_slice()) as *const T, len)
}

/// Runs an FFI body returning a `(buffer, element count)` pair, converting any Rust panic into a
/// null/zero result instead of letting it unwind across the C ABI (unwinding through `extern "C"`
/// aborts the process). `*out_size` receives the count, or 0 on panic. The returned buffer must be
/// reclaimed by the matching `thapthim_free_*`. A null `out_size` yields null without running
/// `body` — the count is the only way the caller can copy out or free the buffer, so a result
/// without it could only leak or corrupt.
fn ffi_array<T>(out_size: *mut i32, body: impl FnOnce() -> (*const T, i32)) -> *const T {
    if out_size.is_null() {
        return std::ptr::null();
    }
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok((ptr, len)) => {
            unsafe { *out_size = len; }
            ptr
        }
        Err(_) => {
            unsafe { *out_size = 0; }
            std::ptr::null()
        }
    }
}

/// `ffi_array` counterpart for entry points returning a single owned pointer (null on panic).
fn ffi_ptr(body: impl FnOnce() -> *mut c_char) -> *mut c_char {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(p) => p,
        Err(_) => std::ptr::null_mut(),
    }
}

/// # Safety
/// `raw_text_ptr` must be null or a valid NUL-terminated C string, and `out_size` a writable
/// `i32` pointer. The returned buffer must be freed with `thapthim_free_array`, passing the
/// length written to `*out_size`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_tcc_positions(
    raw_text_ptr: *const c_char,
    out_size: *mut i32,
) -> *const i32 {
    ffi_array(out_size, || {
        if raw_text_ptr.is_null() {
            return (std::ptr::null(), 0);
        }
        let text_cow = unsafe { read_utf8(raw_text_ptr) };
        into_ffi_buffer(get_tcc().find_positions(&text_cow))
    })
}

/// Word-segmentation FFI entry point. Despite the asset name (`joint_lm` = the three KN bigram
/// layers co-packaged in one file), the decode is a word-first *cascade*, not a joint word⊗syllable
/// optimization: a forward Kneser-Ney bigram Viterbi runs over a single TCC-grid lattice that mixes
/// dictionary-word edges with one-cluster character fallback (so the dict-word vs OOV-run choice is
/// made in that one pass), then only the resulting OOV spans are syllabified in a lazy second pass,
/// then the branching-entropy OOV-merge post-pass runs. Returns a flat array of packed u64 tokens.
/// (Fully in-vocabulary text triggers no syllable work at all — see `segment_words`.)
///
/// `keep_whitespace == 0` drops whitespace-only tokens from the stream (the language-level
/// default); nonzero keeps them, making the tokens a lossless tiling of the input.
///
/// # Safety
/// `raw_text_ptr` must be null or a valid NUL-terminated C string, and `out_size` a writable
/// `i32` pointer. The returned buffer must be freed with `thapthim_free_u64_array`, passing the
/// length written to `*out_size`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_segment(
    raw_text_ptr: *const c_char,
    out_size: *mut i32,
    keep_whitespace: i32,
) -> *const u64 {
    ffi_array(out_size, || {
        if raw_text_ptr.is_null() {
            return (std::ptr::null(), 0);
        }
        let text_cow = unsafe { read_utf8(raw_text_ptr) };
        // Shared global engine (O(1) after bootstrap); word-only Viterbi (no syllable work unless
        // the input has OOV spans).
        let mut packed_tokens = get_engine().segment_words(&text_cow);
        if keep_whitespace == 0 {
            packed_tokens.retain(|&t| !is_whitespace_token(&text_cow, t));
        }
        into_ffi_buffer(packed_tokens)
    })
}

/// Companion to `thapthim_segment`: returns the syllable-level token stream for the same text,
/// each token packed identically as [ Start | Length | Tier ]. Syllable boundaries are a
/// superset of the word boundaries returned by `thapthim_segment`. `keep_whitespace` behaves as
/// in `thapthim_segment`.
///
/// # Safety
/// Same contract as `thapthim_segment`: `raw_text_ptr` null or a valid NUL-terminated C string,
/// `out_size` writable; free the result with `thapthim_free_u64_array`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_segment_syllables(
    raw_text_ptr: *const c_char,
    out_size: *mut i32,
    keep_whitespace: i32,
) -> *const u64 {
    ffi_array(out_size, || {
        if raw_text_ptr.is_null() {
            return (std::ptr::null(), 0);
        }
        let text_cow = unsafe { read_utf8(raw_text_ptr) };
        let mut packed_tokens = get_engine().segment_syllables(&text_cow);
        if keep_whitespace == 0 {
            packed_tokens.retain(|&t| !is_whitespace_token(&text_cow, t));
        }
        into_ffi_buffer(packed_tokens)
    })
}

/// Normalizes Thai text (shared `std_normalize`) and returns a freshly allocated, NUL-terminated
/// C string the caller must free with `thapthim_free_string`. Returns null on a null input or if
/// the result somehow contains an interior NUL.
///
/// # Safety
/// `raw_text_ptr` must be null or a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_normalize(raw_text_ptr: *const c_char) -> *mut c_char {
    ffi_ptr(|| {
        if raw_text_ptr.is_null() {
            return std::ptr::null_mut();
        }
        let text_cow = unsafe { read_utf8(raw_text_ptr) };
        let normalized = crate::normalize::std_normalize(&text_cow);
        match CString::new(normalized) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

/// Returns 1 if the input is a correctly-spelled word (a dictionary entry after normalization),
/// else 0 (also 0 on null input or a panic).
///
/// # Safety
/// `raw_text_ptr` must be null or a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_spell(raw_text_ptr: *const c_char) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if raw_text_ptr.is_null() {
            return false;
        }
        let text_cow = unsafe { read_utf8(raw_text_ptr) };
        get_spell().is_word(&text_cow)
    }));
    matches!(result, Ok(true)) as i32
}

/// Ranked spelling suggestions for the input word, newline-joined into one C string (empty string
/// if there are none). Free with `thapthim_free_string`.
///
/// # Safety
/// `raw_text_ptr` must be null or a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_suggest(raw_text_ptr: *const c_char) -> *mut c_char {
    ffi_ptr(|| {
        if raw_text_ptr.is_null() {
            return std::ptr::null_mut();
        }
        let text_cow = unsafe { read_utf8(raw_text_ptr) };
        let joined = get_spell()
            .suggest(&text_cow, crate::spell::SUGGEST_TOP_N)
            .join("\n");
        match CString::new(joined) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

/// Corrects a single word: a valid word is returned unchanged (normalized), an unknown word becomes
/// its top-ranked candidate (or is left unchanged if none is close enough). Free with
/// `thapthim_free_string`.
///
/// # Safety
/// `raw_text_ptr` must be null or a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_correct(raw_text_ptr: *const c_char) -> *mut c_char {
    ffi_ptr(|| {
        if raw_text_ptr.is_null() {
            return std::ptr::null_mut();
        }
        let text_cow = unsafe { read_utf8(raw_text_ptr) };
        let corrected = get_spell().correct(&text_cow);
        match CString::new(corrected) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

/// Context-aware sentence correction over a pre-segmented token sequence. Tokens arrive concatenated
/// (`concat_ptr`) with a per-token byte-length array (`lengths_ptr`), exactly like `thapthim_pos_tag`.
/// A bigram Viterbi (word-layer KN LM) corrects the tokens jointly; the corrected surfaces are
/// returned concatenated in one C string, with each corrected token's byte length written into the
/// caller-provided `out_lengths_ptr` buffer (which must hold `n_tokens` `i32`s). There is exactly one
/// output token per input token, so the caller already knows the count. Free the string with
/// `thapthim_free_string`. `THAPTHIM_SPELL_SENT_LAMBDA` overrides the edit-vs-bigram weight.
///
/// # Safety
/// `concat_ptr` must point to `sum(lengths)` readable bytes; `lengths_ptr` and `out_lengths_ptr` to
/// `n_tokens` readable/writable `i32`s. Free the returned string with `thapthim_free_string`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_correct_sent(
    concat_ptr: *const c_char,
    lengths_ptr: *const i32,
    n_tokens: i32,
    out_lengths_ptr: *mut i32,
) -> *mut c_char {
    ffi_ptr(|| {
        if concat_ptr.is_null() || lengths_ptr.is_null() || out_lengths_ptr.is_null() || n_tokens <= 0 {
            return std::ptr::null_mut();
        }
        let concat = unsafe { read_utf8(concat_ptr) };
        let lengths = unsafe { std::slice::from_raw_parts(lengths_ptr, n_tokens as usize) };
        let bytes = concat.as_bytes();
        let mut tokens: Vec<&str> = Vec::with_capacity(n_tokens as usize);
        let mut off = 0usize;
        for &len in lengths {
            let end = (off + len.max(0) as usize).min(bytes.len());
            tokens.push(std::str::from_utf8(&bytes[off..end]).unwrap_or(""));
            off = end;
        }

        let sent_lambda = std::env::var("THAPTHIM_SPELL_SENT_LAMBDA")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(crate::spell::DEFAULT_SENT_LAMBDA);
        // Context bigram source: default = the thai_words-aligned bigram (covers dict words the
        // LST20 bigram lacks); THAPTHIM_SPELL_BIGRAM=lst20 opts out to the segmenter's LST20 LM.
        let spell = get_spell();
        let engine = get_engine();
        let corrected = match std::env::var("THAPTHIM_SPELL_BIGRAM").ok().as_deref() {
            Some("lst20") => spell.correct_sentence(&tokens, sent_lambda, |w1, w2| {
                engine.score_transition(&crate::lattice::LatticeTier::Word, w1, w2)
            }),
            _ => spell.correct_sentence(&tokens, sent_lambda, |w1, w2| {
                spell.bigram_logp(w1, w2)
            }),
        };

        let out = unsafe { std::slice::from_raw_parts_mut(out_lengths_ptr, n_tokens as usize) };
        let mut result = String::new();
        for (i, tok) in corrected.iter().take(n_tokens as usize).enumerate() {
            out[i] = tok.len() as i32;
            result.push_str(tok);
        }
        match CString::new(result) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    })
}

/// POS-tags an already-segmented token sequence. Tokens arrive as one concatenated UTF-8 buffer plus
/// a per-token byte-length array — not a delimited string, because a whitespace token can itself be a
/// space or newline, so no byte is safe as a separator. Returns a freshly allocated array of exactly
/// `n_tokens` tag ids (dense `Pos` ids, 0..16, in `Pos::ALL` order), which the caller must free with
/// `thapthim_free_u8_array`. The model is loaded once from `THAPTHIM_POS_MODEL` on first call.
///
/// This is the pipeline (cascade) surface: the caller segments first (the Ruby layer calls
/// `word_segment`), then hands the tokens here. The tagger itself never segments.
///
/// # Safety
/// `concat_ptr` must point to at least `sum(lengths)` readable bytes; `lengths_ptr` to `n_tokens`
/// readable `i32`s; `out_size` must be writable. Free the result with `thapthim_free_u8_array`,
/// passing the length written to `*out_size`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_pos_tag(
    concat_ptr: *const c_char,
    lengths_ptr: *const i32,
    n_tokens: i32,
    out_size: *mut i32,
) -> *const u8 {
    ffi_array(out_size, || {
        if concat_ptr.is_null() || lengths_ptr.is_null() || n_tokens <= 0 {
            return (std::ptr::null(), 0);
        }
        let n = n_tokens as usize;
        let lengths = unsafe { std::slice::from_raw_parts(lengths_ptr, n) };
        let total: usize = lengths.iter().map(|&l| l.max(0) as usize).sum();
        let bytes = unsafe { std::slice::from_raw_parts(concat_ptr as *const u8, total) };

        // Split the flat buffer back into tokens by the given byte-lengths. `off + l <= total` always
        // (total is their sum), so the slice never goes out of bounds. Ruby hands clean UTF-8; a
        // mismatched length that splits a codepoint yields "" rather than aborting across the ABI.
        let mut tokens: Vec<&str> = Vec::with_capacity(n);
        let mut off = 0usize;
        for &l in lengths {
            let l = l.max(0) as usize;
            tokens.push(std::str::from_utf8(&bytes[off..off + l]).unwrap_or(""));
            off += l;
        }

        let ids: Vec<u8> =
            get_pos_tagger().tag(&tokens).into_iter().map(|t| t.idx() as u8).collect();
        into_ffi_buffer(ids)
    })
}

/// Frees a `u8` tag-id array returned by `thapthim_pos_tag`.
///
/// # Safety
/// `ptr` and `size` must be exactly a pointer and element count previously returned by
/// `thapthim_pos_tag`, freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_free_u8_array(ptr: *mut u8, size: i32) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, size as usize));
        }
    }
}

/// Frees a string returned by `thapthim_normalize`.
///
/// # Safety
/// `ptr` must be null or a pointer previously returned by `thapthim_normalize`, freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

/// # Safety
/// `ptr` and `size` must be exactly a pointer and element count previously returned by
/// `thapthim_tcc_positions`, freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_free_array(ptr: *mut i32, size: i32) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, size as usize));
        }
    }
}

/// Specialized complementary function to safely free u64 packed token streams
///
/// # Safety
/// `ptr` and `size` must be exactly a pointer and element count previously returned by
/// `thapthim_segment` or `thapthim_segment_syllables`, freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_free_u64_array(ptr: *mut u64, size: i32) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, size as usize));
        }
    }
}