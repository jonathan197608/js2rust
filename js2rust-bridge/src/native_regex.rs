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
/// str.match(regex) with /g flag → string[] | null
///
/// Returns all matches as NUL-separated substrings in a single Zig Arena buffer.
/// `out_count` receives the number of matches — 0 means no match (null in JS).
/// Unlike host_regex_match, this function does not include capture groups.
fn host_regex_match_global_inner(pattern: HostStr, text: HostStr) -> (String, usize) {
    let re = match fancy_regex::Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return (String::new(), 0),
    };
    let mut result = String::new();
    let mut count: usize = 0;
    // fancy-regex Regex doesn't have find_iter(), so we manually find all matches
    let mut search_start: usize = 0;
    while let Ok(Some(m)) = re.find(&text[search_start..]) {
        if count > 0 {
            result.push('\0');
        }
        // m.start() is relative to the searched slice, so add search_start
        let absolute_start: usize = search_start + m.start();
        let absolute_end: usize = search_start + m.end();
        result.push_str(&text[absolute_start..absolute_end]);
        count += 1;
        // Move search start past this match
        search_start = absolute_end;
        // If the match is empty, move forward to avoid infinite loop
        if m.start() == m.end() {
            search_start += 1;
        }
        // Check if we've reached the end
        if search_start >= text.len() {
            break;
        }
    }
    (result, count)
}

/// # Safety
///
/// Called from Zig via C ABI. ptr/len must be valid. out_count must be a valid pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_regex_match_global(
    pattern_ptr: *const u8,
    pattern_len: usize,
    text_ptr: *const u8,
    text_len: usize,
    out_count: *mut usize,
) -> JsStr {
    let pattern = unsafe { HostStr::from_raw(pattern_ptr, pattern_len) };
    let text = unsafe { HostStr::from_raw(text_ptr, text_len) };
    let (result_str, count) = host_regex_match_global_inner(pattern, text);
    unsafe {
        *out_count = count;
    }
    if result_str.is_empty() {
        JsStr::empty()
    } else {
        JsStr::new(&result_str)
    }
}

/// str.matchAll(regex) → string[][] (array of match arrays with capture groups)
///
/// Returns all matches with capture groups as NUL-separated substrings.
/// Groups within each match are NUL-separated; matches are concatenated sequentially.
/// `out_match_count` receives the number of matches.
/// `out_group_count` receives the number of groups per match (including full match at index 0).
/// Both counts are 0 if no match.
fn host_regex_match_all_inner(pattern: HostStr, text: HostStr) -> (String, usize, usize) {
    let re = match fancy_regex::Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return (String::new(), 0, 0),
    };
    let mut result = String::new();
    let mut match_count: usize = 0;
    let mut group_count: usize = 0;
    let mut search_start: usize = 0;
    while let Ok(Some(caps)) = re.captures_from_pos(&text, search_start) {
        // Record group count from first match (all matches have same count)
        if match_count == 0 {
            group_count = caps.len();
        }
        for i in 0..caps.len() {
            if !result.is_empty() {
                result.push('\0');
            }
            if let Some(m) = caps.get(i) {
                result.push_str(m.as_str());
            }
            // Missing groups produce empty string (NUL-separated placeholder)
        }
        match_count += 1;
        // Advance past this match
        let full_match = caps
            .get(0)
            .expect("full match group is guaranteed by captures_from_pos returning Some");
        search_start = full_match.end();
        // If empty match, advance by 1 to avoid infinite loop
        if full_match.start() == full_match.end() {
            search_start += 1;
        }
        if search_start >= text.len() {
            break;
        }
    }
    (result, match_count, group_count)
}

/// # Safety
///
/// Called from Zig via C ABI. ptr/len must be valid. out_count must be a valid pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_regex_match_all(
    pattern_ptr: *const u8,
    pattern_len: usize,
    text_ptr: *const u8,
    text_len: usize,
    out_match_count: *mut usize,
    out_group_count: *mut usize,
) -> JsStr {
    let pattern = unsafe { HostStr::from_raw(pattern_ptr, pattern_len) };
    let text = unsafe { HostStr::from_raw(text_ptr, text_len) };
    let (result_str, match_count, group_count) = host_regex_match_all_inner(pattern, text);
    unsafe {
        *out_match_count = match_count;
        *out_group_count = group_count;
    }
    if result_str.is_empty() {
        JsStr::empty()
    } else {
        JsStr::new(&result_str)
    }
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
        JsStr::new(&result_str)
    }
}
