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

// Python (PyO3) binding — same engine, different outer layer. Gated so the default Ruby build is
// untouched and rb-sys/pyo3 never coexist in one compile.
#[cfg(feature = "python")]
mod python;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::OnceLock;
use crate::tcc::TccSegmenter;
use crate::lattice::RuntimeEngine;

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

/// Reads a (non-null) C string pointer as UTF-8, replacing any invalid bytes with
/// U+FFFD instead of discarding the entire input. The Ruby layer already hands us
/// clean UTF-8, so this is defense-in-depth for direct C-ABI callers (e.g. future
/// language bindings) — a single bad byte should never blank out the whole result.
unsafe fn read_utf8<'a>(ptr: *const c_char) -> std::borrow::Cow<'a, str> {
    String::from_utf8_lossy(unsafe { CStr::from_ptr(ptr) }.to_bytes())
}

/// Runs an FFI body returning a `(buffer, element count)` pair, converting any Rust panic into a
/// null/zero result instead of letting it unwind across the C ABI (unwinding through `extern "C"`
/// aborts the process). `*out_size` receives the count, or 0 on panic. The returned buffer must be
/// reclaimed by the matching `thapthim_free_*`.
fn ffi_array<T>(out_size: *mut i32, body: impl FnOnce() -> (*const T, i32)) -> *const T {
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
        let positions = get_tcc().find_positions(&text_cow);
        let len = positions.len() as i32;
        (Box::into_raw(positions.into_boxed_slice()) as *const i32, len)
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
/// # Safety
/// `raw_text_ptr` must be null or a valid NUL-terminated C string, and `out_size` a writable
/// `i32` pointer. The returned buffer must be freed with `thapthim_free_u64_array`, passing the
/// length written to `*out_size`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_segment(
    raw_text_ptr: *const c_char,
    out_size: *mut i32,
) -> *const u64 {
    ffi_array(out_size, || {
        if raw_text_ptr.is_null() {
            return (std::ptr::null(), 0);
        }
        let text_cow = unsafe { read_utf8(raw_text_ptr) };
        // Shared global engine (O(1) after bootstrap); word-only Viterbi (no syllable work unless
        // the input has OOV spans).
        let packed_tokens = get_engine().segment_words(&text_cow);
        let len = packed_tokens.len() as i32;
        (Box::into_raw(packed_tokens.into_boxed_slice()) as *const u64, len)
    })
}

/// Companion to `thapthim_segment`: returns the syllable-level token stream for the same text,
/// each token packed identically as [ Start | Length | Tier ]. Syllable boundaries are a
/// superset of the word boundaries returned by `thapthim_segment`.
///
/// # Safety
/// Same contract as `thapthim_segment`: `raw_text_ptr` null or a valid NUL-terminated C string,
/// `out_size` writable; free the result with `thapthim_free_u64_array`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn thapthim_segment_syllables(
    raw_text_ptr: *const c_char,
    out_size: *mut i32,
) -> *const u64 {
    ffi_array(out_size, || {
        if raw_text_ptr.is_null() {
            return (std::ptr::null(), 0);
        }
        let text_cow = unsafe { read_utf8(raw_text_ptr) };
        let packed_tokens = get_engine().segment_syllables(&text_cow);
        let len = packed_tokens.len() as i32;
        (Box::into_raw(packed_tokens.into_boxed_slice()) as *const u64, len)
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