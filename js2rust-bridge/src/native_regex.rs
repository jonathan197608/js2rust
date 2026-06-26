//! Host functions for RegExp support via fancy-regex.
//!
//! These functions are called from generated Zig code via C ABI.
//! Wrappers are written manually (not `#[host_fn]`) because that proc-macro
//! is designed for external crates and references `js2rust_bridge::sdk::HostStr`
//! which doesn't resolve from within the `js2rust-bridge` crate itself.

use crate::sdk::{HostStr, JsStr};

/// regex.test(str) → bool
/// Returns true if the pattern matches any part of the text.
/// On pattern compilation error, returns false (JS behavior: no exception for test).
fn host_regex_test_inner(pattern: HostStr, text: HostStr) -> bool {
    fancy_regex::Regex::new(&pattern)
        .ok()
        .and_then(|re| re.is_match(&text).ok())
        .unwrap_or(false)
}

/// # Safety
///
/// Called from Zig via C ABI. ptr/len must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_regex_test(
    pattern_ptr: *const u8,
    pattern_len: usize,
    text_ptr: *const u8,
    text_len: usize,
) -> bool {
    let pattern = unsafe { HostStr::from_raw(pattern_ptr, pattern_len) };
    let text = unsafe { HostStr::from_raw(text_ptr, text_len) };
    host_regex_test_inner(pattern, text)
}

/// str.search(regex) → i64
/// Returns the index of the first match, or -1 if no match is found.
/// On pattern compilation error, returns -1 (JS behavior for exotic edge cases).
fn host_regex_search_inner(pattern: HostStr, text: HostStr) -> i64 {
    fancy_regex::Regex::new(&pattern)
        .ok()
        .and_then(|re| re.find(&text).ok())
        .flatten()
        .map(|m| m.start() as i64)
        .unwrap_or(-1)
}

/// # Safety
///
/// Called from Zig via C ABI. ptr/len must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_regex_search(
    pattern_ptr: *const u8,
    pattern_len: usize,
    text_ptr: *const u8,
    text_len: usize,
) -> i64 {
    let pattern = unsafe { HostStr::from_raw(pattern_ptr, pattern_len) };
    let text = unsafe { HostStr::from_raw(text_ptr, text_len) };
    host_regex_search_inner(pattern, text)
}

/// str.match(regex) → string[] | null
///
/// Returns match results as NUL-separated substrings in a single Zig Arena buffer.
/// Index 0 is the full match, indices 1+ are capture groups.
/// `out_count` receives the number of substrings — 0 means no match (null in JS).
fn host_regex_match_inner(pattern: HostStr, text: HostStr) -> (String, usize) {
    let re = match fancy_regex::Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return (String::new(), 0),
    };
    let caps = match re.captures(&text) {
        Ok(Some(c)) => c,
        _ => return (String::new(), 0),
    };
    let mut result = String::new();
    let mut count = 0;
    for i in 0..caps.len() {
        if let Some(m) = caps.get(i) {
            if count > 0 {
                result.push('\0');
            }
            result.push_str(m.as_str());
            count += 1;
        }
    }
    (result, count)
}

/// # Safety
///
/// Called from Zig via C ABI. ptr/len must be valid. out_count must be a valid pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_regex_match(
    pattern_ptr: *const u8,
    pattern_len: usize,
    text_ptr: *const u8,
    text_len: usize,
    out_count: *mut usize,
) -> JsStr {
    let pattern = unsafe { HostStr::from_raw(pattern_ptr, pattern_len) };
    let text = unsafe { HostStr::from_raw(text_ptr, text_len) };
    let (result_str, count) = host_regex_match_inner(pattern, text);
    unsafe {
        *out_count = count;
    }
    if result_str.is_empty() {
        JsStr::empty()
    } else {
        // Use Box::leak to avoid extern "C" dependency on js_allocator_dupe.
        // This leaks heap memory, but avoids linker errors when js2rust-bridge
        // is compiled as a build dependency (no Zig runtime available).
        // TODO: switch to js_allocator_dupe when bridge is refactored to
        // separate build-time and runtime crates.
        let leaked: &'static [u8] = Box::leak(result_str.into_bytes().into_boxed_slice());
        JsStr {
            ptr: leaked.as_ptr(),
            len: leaked.len() as isize,
        }
    }
}
