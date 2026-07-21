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
/// Unmatched groups are included as empty strings (NUL-separated placeholders)
/// so that the Zig-side parser can reconstruct the full array with correct indices.
/// `out_count` receives the number of substrings (always = caps.len() when matched) —
/// 0 means no match (null in JS).
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
    let mut is_first = true;
    for i in 0..caps.len() {
        if !is_first {
            result.push('\0');
        }
        is_first = false;
        // Unmatched groups produce empty string (placeholder for JS undefined)
        if let Some(m) = caps.get(i) {
            result.push_str(m.as_str());
        }
    }
    (result, caps.len())
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
/// Unmatched groups are included as empty strings (NUL-separated placeholders)
/// so that the Zig-side parser can reconstruct each match array with correct indices.
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
    let mut is_first_segment = true; // tracks whether this is the first NUL-separated segment overall
    let mut search_start: usize = 0;
    while let Ok(Some(caps)) = re.captures_from_pos(&text, search_start) {
        // Record group count from first match (all matches have same count)
        if match_count == 0 {
            group_count = caps.len();
        }
        for i in 0..caps.len() {
            if !is_first_segment {
                result.push('\0');
            }
            is_first_segment = false;
            // Unmatched groups produce empty string (placeholder for JS undefined)
            if let Some(m) = caps.get(i) {
                result.push_str(m.as_str());
            }
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

/// str.replace(regex, replacement) → string
///
/// Replaces the first match of the regex pattern with the replacement string.
/// On pattern compilation error or no match, returns the original text.
fn host_regex_replace_inner(pattern: HostStr, text: HostStr, replacement: HostStr) -> String {
    match fancy_regex::Regex::new(&pattern) {
        Ok(re) => re.replace(&text, replacement.as_ref()).to_string(),
        Err(_) => text.to_string(),
    }
}

/// # Safety
///
/// Called from Zig via C ABI. ptr/len must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_regex_replace(
    pattern_ptr: *const u8,
    pattern_len: usize,
    text_ptr: *const u8,
    text_len: usize,
    replacement_ptr: *const u8,
    replacement_len: usize,
) -> JsStr {
    let pattern = unsafe { HostStr::from_raw(pattern_ptr, pattern_len) };
    let text = unsafe { HostStr::from_raw(text_ptr, text_len) };
    let replacement = unsafe { HostStr::from_raw(replacement_ptr, replacement_len) };
    let result = host_regex_replace_inner(pattern, text, replacement);
    if result.is_empty() {
        JsStr::empty()
    } else {
        JsStr::new(&result)
    }
}

/// str.replaceAll(regex, replacement) → string
///
/// Replaces all matches of the regex pattern with the replacement string.
/// On pattern compilation error or no match, returns the original text.
fn host_regex_replace_all_inner(pattern: HostStr, text: HostStr, replacement: HostStr) -> String {
    match fancy_regex::Regex::new(&pattern) {
        Ok(re) => re.replace_all(&text, replacement.as_ref()).to_string(),
        Err(_) => text.to_string(),
    }
}

/// # Safety
///
/// Called from Zig via C ABI. ptr/len must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_regex_replace_all(
    pattern_ptr: *const u8,
    pattern_len: usize,
    text_ptr: *const u8,
    text_len: usize,
    replacement_ptr: *const u8,
    replacement_len: usize,
) -> JsStr {
    let pattern = unsafe { HostStr::from_raw(pattern_ptr, pattern_len) };
    let text = unsafe { HostStr::from_raw(text_ptr, text_len) };
    let replacement = unsafe { HostStr::from_raw(replacement_ptr, replacement_len) };
    let result = host_regex_replace_all_inner(pattern, text, replacement);
    if result.is_empty() {
        JsStr::empty()
    } else {
        JsStr::new(&result)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Safely create a HostStr from a Rust string for testing.
    fn hs(s: &str) -> HostStr<'_> {
        // SAFETY: s is a valid Rust string reference with valid UTF-8.
        unsafe { HostStr::from_raw(s.as_ptr(), s.len()) }
    }

    #[test]
    fn match_inner_includes_unmatched_groups_as_empty() {
        // Pattern with 2 capture groups: (\d)(\w)?
        // Text "3" — group 2 (\w)? is optional and won't match.
        let (result, count) = host_regex_match_inner(hs(r"(\d)(\w)?"), hs("3"));
        assert_eq!(count, 3); // full match + 2 groups (including unmatched)
        // Result should be "3\0\0" — full match "3", group 1 "3", group 2 "" (unmatched)
        let parts: Vec<&str> = result.split('\0').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "3"); // full match
        assert_eq!(parts[1], "3"); // group 1 (\d)
        assert_eq!(parts[2], ""); // group 2 (\w)? — unmatched → empty
    }

    #[test]
    fn match_inner_all_groups_matched() {
        let (result, count) = host_regex_match_inner(hs(r"(\d)(\w)"), hs("3a"));
        assert_eq!(count, 3);
        let parts: Vec<&str> = result.split('\0').collect();
        assert_eq!(parts, vec!["3a", "3", "a"]);
    }

    #[test]
    fn match_inner_no_match_returns_empty() {
        let (result, count) = host_regex_match_inner(hs(r"(\d)"), hs("abc"));
        assert_eq!(count, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn match_all_inner_includes_unmatched_groups() {
        // Pattern (\d)(\w)? with /g — group 2 is optional.
        // Text "3a5" — first match: "3a" (both groups), second match: "5" (group 2 unmatched)
        let (result, match_count, group_count) =
            host_regex_match_all_inner(hs(r"(\d)(\w)?"), hs("3a5"));
        assert_eq!(match_count, 2);
        assert_eq!(group_count, 3); // full + 2 groups
        // 2 matches × 3 groups = 6 segments
        let parts: Vec<&str> = result.split('\0').collect();
        assert_eq!(parts.len(), 6);
        // First match: "3a", "3", "a"
        assert_eq!(parts[0], "3a");
        assert_eq!(parts[1], "3");
        assert_eq!(parts[2], "a");
        // Second match: "5", "5", "" (group 2 unmatched → empty)
        assert_eq!(parts[3], "5");
        assert_eq!(parts[4], "5");
        assert_eq!(parts[5], "");
    }

    #[test]
    fn match_all_inner_empty_string_group() {
        // Pattern (\d*) with /g — group 1 can match empty string.
        // This tests that empty-string matches are correctly NUL-separated.
        let (result, match_count, group_count) = host_regex_match_all_inner(hs(r"(\d*)"), hs("ab"));
        // \d* matches empty at positions 0, 1, 2 (before 'a', between 'a'/'b', after 'b')
        assert!(match_count >= 1);
        assert_eq!(group_count, 2); // full match + 1 group
        let parts: Vec<&str> = result.split('\0').collect();
        // Each match has 2 segments (full + group), all should be empty strings
        for part in &parts {
            assert_eq!(*part, "");
        }
        assert_eq!(parts.len(), match_count * group_count);
    }
}
