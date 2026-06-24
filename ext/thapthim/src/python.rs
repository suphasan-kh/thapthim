// ext/thapthim/src/python.rs
//
// PyO3 binding for the Thapthim engine. This is the Python counterpart to the C-ABI shim in
// `lib.rs` (which serves the Ruby/Fiddle layer): both call the *same* `RuntimeEngine`, cached
// `TccSegmenter`, and `normalize::std_normalize`, so behaviour is identical across languages.
//
// Unlike the C-ABI path, nothing crosses a NUL-terminated boundary here — Python `str` is handed
// to Rust as a guaranteed-UTF-8 `&str` and passed straight to the engine, so the encoding/NUL
// sanitisation the Ruby layer does (`sanitize_input`) is unnecessary.
//
// Decode strategy: the heavy engine work runs with the GIL released; the resulting tokens are then
// materialised *directly* into Python `str` objects from the source byte slices (one copy each, no
// intermediate Rust `String`). This is leaner than collecting a `Vec<String>` and letting PyO3
// re-copy it. `from_utf8_lossy` is used (not `_unchecked`): engine boundaries are always valid
// UTF-8 boundaries so it borrows without allocating in practice, but it stays memory-safe even if
// that invariant were ever violated — no `unsafe` anywhere in this binding.

use pyo3::prelude::*;
use pyo3::types::PyString;
use pyo3::wrap_pyfunction;
use rayon::prelude::*;

use crate::get_engine;
use crate::get_tcc;
use crate::normalize::std_normalize as rust_normalize;

// Each engine token packs [ Start:32 | Length:24 | Tier:8 ] as UTF-8 *byte* offsets into the source.
#[inline]
fn unpack(tok: u64) -> (usize, usize) {
    let start = (tok >> 32) as usize;
    let len = ((tok >> 8) & 0xFF_FFFF) as usize;
    (start, len)
}

// Materialise packed tokens straight into a list of Python `str`, slicing the source bytes. Must be
// called with the GIL held (it allocates Python objects). See the module header for why this beats
// returning `Vec<String>`.
fn decode_pystrings<'py>(py: Python<'py>, text: &str, packed: &[u64]) -> Vec<Bound<'py, PyString>> {
    let bytes = text.as_bytes();
    packed
        .iter()
        .map(|&tok| {
            let (start, len) = unpack(tok);
            // Engine tokens span TCC boundaries (valid UTF-8 boundaries), so this is Borrowed in
            // practice — no alloc. Lossy keeps it safe regardless; the byte-slice index is itself
            // bounds-checked (a bad offset would panic → Python exception, never UB).
            let s = String::from_utf8_lossy(&bytes[start..start + len]);
            PyString::new(py, s.as_ref())
        })
        .collect()
}

// String-returning decode, used by the batch path (which builds its strings off-GIL under rayon and
// so cannot create Python objects directly).
fn decode_packed(text: &str, packed: &[u64]) -> Vec<String> {
    let bytes = text.as_bytes();
    packed
        .iter()
        .map(|&tok| {
            let (start, len) = unpack(tok);
            String::from_utf8_lossy(&bytes[start..start + len]).into_owned()
        })
        .collect()
}

/// Thai text normalization (parity with the Ruby `std_normalize`). See `crate::normalize`.
#[pyfunction]
fn std_normalize(py: Python<'_>, text: &str) -> String {
    let owned = text.to_owned();
    py.allow_threads(move || rust_normalize(&owned))
}

/// Word-level segmentation. Returns the list of word substrings.
///
/// `normalize=True` runs `std_normalize` first (the returned tokens are then substrings of the
/// normalized text, not the original) — matching the Ruby `segment(text, normalize: true)`.
#[pyfunction]
#[pyo3(signature = (text, normalize=false))]
fn segment<'py>(py: Python<'py>, text: &str, normalize: bool) -> Vec<Bound<'py, PyString>> {
    let owned = text.to_owned();
    // Release the GIL across the (Python-free) heavy work: the engine is a read-only `&'static`
    // singleton, so N Python threads can run segmentation on N cores concurrently.
    let (text, packed) = py.allow_threads(move || {
        let text = if normalize { rust_normalize(&owned) } else { owned };
        let packed = get_engine().segment_words(&text);
        (text, packed)
    });
    decode_pystrings(py, &text, &packed)
}

/// Syllable-level segmentation. Boundaries are a superset of `segment`'s word boundaries.
/// See `segment` for `normalize`.
#[pyfunction]
#[pyo3(signature = (text, normalize=false))]
fn syllables<'py>(py: Python<'py>, text: &str, normalize: bool) -> Vec<Bound<'py, PyString>> {
    let owned = text.to_owned();
    let (text, packed) = py.allow_threads(move || {
        let text = if normalize { rust_normalize(&owned) } else { owned };
        let packed = get_engine().segment_syllables(&text);
        (text, packed)
    });
    decode_pystrings(py, &text, &packed)
}

/// TCC (Thai Character Cluster) segmentation. Returns the list of unbreakable cluster substrings.
#[pyfunction]
fn tcc_segment<'py>(py: Python<'py>, text: &str) -> Vec<Bound<'py, PyString>> {
    let owned = text.to_owned();
    let (owned, positions) =
        py.allow_threads(move || {
            let positions = get_tcc().find_byte_positions(&owned);
            (owned, positions)
        });
    let bytes = owned.as_bytes();
    positions
        .windows(2)
        .map(|w| {
            // TCC byte positions fall on valid UTF-8 boundaries → Borrowed in practice; lossy
            // keeps it safe regardless, with no `unsafe`.
            let s = String::from_utf8_lossy(&bytes[w[0]..w[1]]);
            PyString::new(py, s.as_ref())
        })
        .collect()
}

/// TCC cluster boundaries as Unicode *character* indices (parity with the Ruby `tcc_positions`,
/// which slices by character). Returns `[0]` for empty input.
#[pyfunction]
fn tcc_positions(py: Python<'_>, text: &str) -> Vec<i32> {
    let owned = text.to_owned();
    py.allow_threads(move || get_tcc().find_positions(&owned))
}

/// Performance lever: word boundaries as raw `(start_byte, length_byte)` tuples, with no per-token
/// string allocation — for benchmarking pure engine throughput or doing your own slicing.
#[pyfunction]
fn segment_offsets(py: Python<'_>, text: &str) -> Vec<(usize, usize)> {
    let owned = text.to_owned();
    py.allow_threads(move || get_engine().segment_words(&owned).iter().map(|&t| unpack(t)).collect())
}

/// Performance lever: word-segment a batch in one boundary crossing. The GIL is released for the
/// whole batch and the items are segmented across cores with rayon (engine is `Sync`), so this
/// scales with available CPUs independently of Python-level threading.
#[pyfunction]
fn segment_batch(py: Python<'_>, texts: Vec<String>) -> Vec<Vec<String>> {
    py.allow_threads(move || {
        let engine = get_engine();
        texts
            .par_iter()
            .map(|t| decode_packed(t, &engine.segment_words(t)))
            .collect()
    })
}

#[pymodule]
fn thapthim(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__doc__", "Thai word/syllable/TCC segmentation over the Thapthim Rust engine.")?;
    m.add_function(wrap_pyfunction!(std_normalize, m)?)?;
    m.add_function(wrap_pyfunction!(segment, m)?)?;
    m.add_function(wrap_pyfunction!(syllables, m)?)?;
    m.add_function(wrap_pyfunction!(tcc_segment, m)?)?;
    m.add_function(wrap_pyfunction!(tcc_positions, m)?)?;
    m.add_function(wrap_pyfunction!(segment_offsets, m)?)?;
    m.add_function(wrap_pyfunction!(segment_batch, m)?)?;
    Ok(())
}
