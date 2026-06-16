// ext/thapthim/src/lib.rs
pub mod tcc; 

use std::ffi::CStr;
use std::os::raw::c_char;
use crate::tcc::TccSegmenter;

#[unsafe(no_mangle)]
pub extern "C" fn thapthim_tcc_positions(
    raw_text_ptr: *const c_char,
    out_size: *mut i32,
) -> *const i32 {
    if raw_text_ptr.is_null() {
        unsafe { *out_size = 0; }
        return std::ptr::null();
    }

    let c_str = unsafe { CStr::from_ptr(raw_text_ptr) };
    let text = c_str.to_str().unwrap_or("");

    let segmenter = TccSegmenter::new();
    let positions = segmenter.find_positions(text);

    unsafe { *out_size = positions.len() as i32; }

    let boxed_slice = positions.into_boxed_slice();
    Box::into_raw(boxed_slice) as *const i32
}

#[unsafe(no_mangle)]
pub extern "C" fn thapthim_free_array(ptr: *mut i32, size: i32) {
    if !ptr.is_null() {
        unsafe {
            let _ = Vec::from_raw_parts(ptr, size as usize, size as usize);
        }
    }
}