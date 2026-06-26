//! Host functions for RegExp support via fancy-regex.
//!
//! These functions are called from generated Zig code via C ABI.
//! Wrappers are written manually (not `#[host_fn]`) because that proc-macro
//! is designed for external crates and references `js2rust_bridge::sdk::HostStr`
//! which doesn't resolve from within the `js2rust-bridge` crate itself.

use crate::sdk::HostStr;

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
