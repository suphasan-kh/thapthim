// ext/thapthim/src/lib.rs
pub mod tcc;
pub mod lattice;
// Shared with build.rs (which uses the String-keyed types + intern_*); the lib only reads the
// interned types, so the rest is "dead" from the lib's view — silence those warnings.
#[allow(dead_code)]
mod lm_format;

use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::OnceLock;
use crate::tcc::TccSegmenter;
use crate::lattice::RuntimeEngine;

// Thread-safe global single allocation instance lock for the 40MB engine matrix
static ENGINE: OnceLock<RuntimeEngine> = OnceLock::new();

fn get_engine() -> &'static RuntimeEngine {
    ENGINE.get_or_init(|| {
        RuntimeEngine::bootstrap()
    })
}

/// Reads a (non-null) C string pointer as UTF-8, replacing any invalid bytes with
/// U+FFFD instead of discarding the entire input. The Ruby layer already hands us
/// clean UTF-8, so this is defense-in-depth for direct C-ABI callers (e.g. future
/// language bindings) — a single bad byte should never blank out the whole result.
unsafe fn read_utf8<'a>(ptr: *const c_char) -> std::borrow::Cow<'a, str> {
    String::from_utf8_lossy(unsafe { CStr::from_ptr(ptr) }.to_bytes())
}

#[unsafe(no_mangle)]
pub extern "C" fn thapthim_tcc_positions(
    raw_text_ptr: *const c_char,
    out_size: *mut i32,
) -> *const i32 {
    if raw_text_ptr.is_null() {
        unsafe { *out_size = 0; }
        return std::ptr::null();
    }

    let text_cow = unsafe { read_utf8(raw_text_ptr) };
    let text: &str = &text_cow;

    let segmenter = TccSegmenter::new();
    let positions = segmenter.find_positions(text);

    unsafe { *out_size = positions.len() as i32; }

    let boxed_slice = positions.into_boxed_slice();
    Box::into_raw(boxed_slice) as *const i32
}

/// Zero-Allocation Joint-Lattice Tokenizer FFI Interface
/// Sweeps text using Bidirectional Viterbi Consensus and returns a flat array of packed u64 tokens
#[unsafe(no_mangle)]
pub extern "C" fn thapthim_segment(
    raw_text_ptr: *const c_char,
    out_size: *mut i32,
) -> *const u64 {
    if raw_text_ptr.is_null() {
        unsafe { *out_size = 0; }
        return std::ptr::null();
    }

    let text_cow = unsafe { read_utf8(raw_text_ptr) };
    let text: &str = &text_cow;

    // Access the shared global engine instantly (O(1) after bootstrap initialization)
    let engine = get_engine();

    // Word-only Viterbi (no syllable work unless the input has OOV spans).
    let packed_tokens = engine.segment_words(text);

    unsafe { *out_size = packed_tokens.len() as i32; }

    let boxed_slice = packed_tokens.into_boxed_slice();
    Box::into_raw(boxed_slice) as *const u64
}

/// Companion to `thapthim_segment`: returns the syllable-level token stream for the same text,
/// each token packed identically as [ Start | Length | Tier ]. Syllable boundaries are a
/// superset of the word boundaries returned by `thapthim_segment`.
#[unsafe(no_mangle)]
pub extern "C" fn thapthim_segment_syllables(
    raw_text_ptr: *const c_char,
    out_size: *mut i32,
) -> *const u64 {
    if raw_text_ptr.is_null() {
        unsafe { *out_size = 0; }
        return std::ptr::null();
    }

    let text_cow = unsafe { read_utf8(raw_text_ptr) };
    let text: &str = &text_cow;

    let engine = get_engine();
    let packed_tokens = engine.segment_syllables(text);

    unsafe { *out_size = packed_tokens.len() as i32; }

    let boxed_slice = packed_tokens.into_boxed_slice();
    Box::into_raw(boxed_slice) as *const u64
}

/// EXPERIMENTAL A/B variant: word segmentation with syllables as first-class lattice candidates
/// (all scored under the word LM). Wired only for evaluation against `thapthim_segment`.
#[unsafe(no_mangle)]
pub extern "C" fn thapthim_segment_joint(
    raw_text_ptr: *const c_char,
    out_size: *mut i32,
) -> *const u64 {
    if raw_text_ptr.is_null() {
        unsafe { *out_size = 0; }
        return std::ptr::null();
    }

    let text_cow = unsafe { read_utf8(raw_text_ptr) };
    let text: &str = &text_cow;

    let engine = get_engine();
    let packed_tokens = engine.segment_words_joint(text);

    unsafe { *out_size = packed_tokens.len() as i32; }

    let boxed_slice = packed_tokens.into_boxed_slice();
    Box::into_raw(boxed_slice) as *const u64
}

#[unsafe(no_mangle)]
pub extern "C" fn thapthim_free_array(ptr: *mut i32, size: i32) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, size as usize));
        }
    }
}

/// Specialized complementary function to safely free u64 packed token streams
#[unsafe(no_mangle)]
pub extern "C" fn thapthim_free_u64_array(ptr: *mut u64, size: i32) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, size as usize));
        }
    }
}