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

/// Expand a replacement string per ECMA-262 Table 52 ($$ $& $` $' $n $nn $<name>).
///
/// Supports every standard JS replacement pattern:
/// - `$$` → literal `$`
/// - `$&` → the full matched substring
/// - `` $` `` → the portion of `text` before the match
/// - `$'` → the portion of `text` after the match
/// - `$1`-`$99` → the nth capture group; empty if the group exists but did
///   not participate; literal `$n` if the group number exceeds
///   `num_captures`
/// - `$<name>` → named capture group value; empty if the group did not
///   participate or does not exist; literal `$` if no closing `>`
///
/// `$0` is NOT a capture reference — it is treated as a literal `$`
/// followed by `0`, matching JS engine behaviour (the full match is
/// accessed via `$&`).
fn expand_replacement(
    text: &str,
    caps: &fancy_regex::Captures,
    num_captures: usize,
    match_start: usize,
    match_end: usize,
    replacement: &str,
) -> String {
    let mut result = String::new();
    let bytes = replacement.as_bytes();
    let mut i: usize = 0;

    while i < bytes.len() {
        if bytes[i] == b'$' {
            if i + 1 >= bytes.len() {
                // Trailing $ — literal
                result.push('$');
                i += 1;
                continue;
            }
            match bytes[i + 1] {
                // $$ → literal $
                b'$' => {
                    result.push('$');
                    i += 2;
                }
                // $& → full match
                b'&' => {
                    result.push_str(&text[match_start..match_end]);
                    i += 2;
                }
                // $` → text before match
                b'`' => {
                    result.push_str(&text[..match_start]);
                    i += 2;
                }
                // $' → text after match
                b'\'' => {
                    result.push_str(&text[match_end..]);
                    i += 2;
                }
                // $1..$9 — capture group reference (1 or 2 digits)
                b'1'..=b'9' => {
                    let n1 = (bytes[i + 1] - b'0') as usize;
                    // Try two-digit first ($nn)
                    if i + 2 < bytes.len() && bytes[i + 2].is_ascii_digit() {
                        let n2 = n1 * 10 + (bytes[i + 2] - b'0') as usize;
                        if n2 <= num_captures {
                            if let Some(m) = caps.get(n2) {
                                result.push_str(m.as_str());
                            }
                            i += 3;
                            continue;
                        }
                    }
                    // Fall back to one-digit ($n)
                    if n1 <= num_captures {
                        if let Some(m) = caps.get(n1) {
                            result.push_str(m.as_str());
                        }
                        i += 2;
                        continue;
                    }
                    // Group doesn't exist → literal $
                    result.push('$');
                    i += 1;
                }
                // $<name> — named capture group
                b'<' => {
                    if let Some(close_rel) = replacement[i + 2..].find('>') {
                        let name = &replacement[i + 2..i + 2 + close_rel];
                        if let Some(m) = caps.name(name) {
                            result.push_str(m.as_str());
                        }
                        i += 2 + close_rel + 1;
                    } else {
                        // No closing > — literal $
                        result.push('$');
                        i += 1;
                    }
                }
                // Any other $X — literal $
                _ => {
                    result.push('$');
                    i += 1;
                }
            }
        } else {
            // Copy literal run until next $
            let start = i;
            while i < bytes.len() && bytes[i] != b'$' {
                i += 1;
            }
            result.push_str(&replacement[start..i]);
        }
    }
    result
}

/// str.replace(regex, replacement) → string
///
/// Replaces the first match of the regex pattern with the expansion of
/// the replacement string per ECMA-262 replacement patterns ($$, $&, $1,
/// $<name>, etc.). On pattern compilation error or no match, returns the
/// original text unchanged.
fn host_regex_replace_inner(pattern: HostStr, text: HostStr, replacement: HostStr) -> String {
    let re = match fancy_regex::Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return text.to_string(),
    };
    match re.captures(&text) {
        Ok(Some(caps)) => {
            let num_captures = caps.len() - 1; // exclude group 0
            let m = caps
                .get(0)
                .expect("captures returning Some guarantees group 0");
            let expanded =
                expand_replacement(&text, &caps, num_captures, m.start(), m.end(), &replacement);
            // Pre-allocate for single allocation
            let mut result = String::with_capacity(text.len() + expanded.len());
            result.push_str(&text[..m.start()]);
            result.push_str(&expanded);
            result.push_str(&text[m.end()..]);
            result
        }
        _ => text.to_string(),
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
/// Replaces all matches of the regex pattern with the expansion of the
/// replacement string per ECMA-262 replacement patterns. On pattern
/// compilation error or no match, returns the original text unchanged.
fn host_regex_replace_all_inner(pattern: HostStr, text: HostStr, replacement: HostStr) -> String {
    let re = match fancy_regex::Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return text.to_string(),
    };
    let mut result = String::new();
    let mut last_end: usize = 0;
    let mut search_pos: usize = 0;

    while search_pos <= text.len() {
        match re.captures_from_pos(&text, search_pos) {
            Ok(Some(caps)) => {
                let num_captures = caps.len() - 1;
                let m = caps
                    .get(0)
                    .expect("captures_from_pos returning Some guarantees group 0");
                // Append text before this match
                result.push_str(&text[last_end..m.start()]);
                // Expand and append replacement
                let expanded = expand_replacement(
                    &text,
                    &caps,
                    num_captures,
                    m.start(),
                    m.end(),
                    &replacement,
                );
                result.push_str(&expanded);
                // Advance past the match
                last_end = m.end();
                search_pos = m.end();
                // Zero-width match: advance past one character to avoid
                // infinite loop. The character will be included in the next
                // iteration's text-before-match copy.
                if m.start() == m.end() {
                    if search_pos < text.len() {
                        search_pos += 1;
                        while search_pos < text.len() && !text.is_char_boundary(search_pos) {
                            search_pos += 1;
                        }
                    } else {
                        // At end of string — done
                        break;
                    }
                }
            }
            _ => break,
        }
    }
    result.push_str(&text[last_end..]);
    result
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

/// regex.exec(str) with lastIndex support
///
/// Searches from `start` position. Returns match results as NUL-separated substrings
/// (same format as host_regex_match), plus the absolute end position of the full match
/// via `out_end` (for updating `lastIndex`).
/// `out_count` receives the number of substrings (0 = no match).
fn host_regex_exec_inner(pattern: HostStr, text: HostStr, start: usize) -> (String, usize, usize) {
    let re = match fancy_regex::Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return (String::new(), 0, 0),
    };
    let caps = match re.captures_from_pos(&text, start) {
        Ok(Some(c)) => c,
        _ => return (String::new(), 0, 0),
    };
    let mut result = String::new();
    let mut is_first = true;
    for i in 0..caps.len() {
        if !is_first {
            result.push('\0');
        }
        is_first = false;
        if let Some(m) = caps.get(i) {
            result.push_str(m.as_str());
        }
    }
    let full_match = caps
        .get(0)
        .expect("full match group is guaranteed by captures_from_pos returning Some");
    (result, caps.len(), full_match.end())
}

/// # Safety
///
/// Called from Zig via C ABI. ptr/len must be valid. out_count and out_end must be valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_regex_exec(
    pattern_ptr: *const u8,
    pattern_len: usize,
    text_ptr: *const u8,
    text_len: usize,
    start: usize,
    out_count: *mut usize,
    out_end: *mut usize,
) -> JsStr {
    let pattern = unsafe { HostStr::from_raw(pattern_ptr, pattern_len) };
    let text = unsafe { HostStr::from_raw(text_ptr, text_len) };
    let (result_str, count, end) = host_regex_exec_inner(pattern, text, start);
    unsafe {
        *out_count = count;
        *out_end = end;
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

    #[test]
    fn exec_inner_finds_first_match_from_start() {
        let (result, count, end) = host_regex_exec_inner(hs(r"(\d+)"), hs("abc123def456"), 0);
        assert_eq!(count, 2); // full match + 1 group
        assert_eq!(end, 6); // "123" ends at index 6
        let parts: Vec<&str> = result.split('\0').collect();
        assert_eq!(parts, vec!["123", "123"]);
    }

    #[test]
    fn exec_inner_finds_match_from_offset() {
        // Search from position 6 (after "123") — should find "456"
        let (result, count, end) = host_regex_exec_inner(hs(r"(\d+)"), hs("abc123def456"), 6);
        assert_eq!(count, 2);
        assert_eq!(end, 12); // "456" ends at index 12
        let parts: Vec<&str> = result.split('\0').collect();
        assert_eq!(parts, vec!["456", "456"]);
    }

    #[test]
    fn exec_inner_no_match_returns_empty() {
        let (result, count, _end) = host_regex_exec_inner(hs(r"(\d+)"), hs("abcdef"), 0);
        assert_eq!(count, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn exec_inner_unmatched_groups_as_empty() {
        // (\d)(\w)? — group 2 is optional
        let (result, count, _end) = host_regex_exec_inner(hs(r"(\d)(\w)?"), hs("3"), 0);
        assert_eq!(count, 3); // full + 2 groups (including unmatched)
        let parts: Vec<&str> = result.split('\0').collect();
        assert_eq!(parts[0], "3"); // full match
        assert_eq!(parts[1], "3"); // group 1
        assert_eq!(parts[2], ""); // group 2 — unmatched
    }

    // ── P1-24: Replacement pattern expansion tests ──

    #[test]
    fn replace_dollar_dollar_produces_literal_dollar() {
        let result = host_regex_replace_inner(hs(r"\d+"), hs("abc123def"), hs("$$"));
        assert_eq!(result, "abc$def");
    }

    #[test]
    fn replace_dollar_amp_produces_full_match() {
        let result = host_regex_replace_inner(hs(r"\d+"), hs("abc123def"), hs("[$&]"));
        assert_eq!(result, "abc[123]def");
    }

    #[test]
    fn replace_dollar_backtick_produces_before_match() {
        let result = host_regex_replace_inner(hs(r"\d+"), hs("abc123def"), hs("$`"));
        assert_eq!(result, "abcabcdef"); // "abc" (before) replaces "123"
    }

    #[test]
    fn replace_dollar_quote_produces_after_match() {
        let result = host_regex_replace_inner(hs(r"\d+"), hs("abc123def"), hs("$'"));
        assert_eq!(result, "abcdefdef"); // "def" (after) replaces "123"
    }

    #[test]
    fn replace_dollar_n_produces_capture_group() {
        let result = host_regex_replace_inner(hs(r"(\d)(\w)"), hs("3a"), hs("$2$1"));
        assert_eq!(result, "a3"); // swap groups
    }

    #[test]
    fn replace_dollar_nn_two_digit_capture_group() {
        // 12 capture groups: $1=a, $2=b, ..., $10=j, $11=k, $12=l
        let pattern = r"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)";
        let text = "abcdefghijkl";
        let result = host_regex_replace_inner(hs(pattern), hs(text), hs("$10$11"));
        assert_eq!(result, "jk"); // group 10 = "j", group 11 = "k"
    }

    #[test]
    fn replace_dollar_n_nonexistent_group_is_literal() {
        // Only 1 capture group; $2 is out of range → should be literal "$2"
        let result = host_regex_replace_inner(hs(r"(\d)"), hs("3"), hs("$1$2"));
        assert_eq!(result, "3$2"); // $1 → "3", $2 → literal "$2"
    }

    #[test]
    fn replace_dollar_zero_is_literal() {
        // $0 is NOT a capture reference — treated as literal $0
        let result = host_regex_replace_inner(hs(r"(\d)"), hs("3"), hs("$0"));
        assert_eq!(result, "$0");
    }

    #[test]
    fn replace_dollar_name_named_capture() {
        let result = host_regex_replace_inner(hs(r"(?<year>\d{4})"), hs("2026"), hs("$<year>!"));
        assert_eq!(result, "2026!");
    }

    #[test]
    fn replace_dollar_name_nonexistent_is_empty() {
        // Named group "year" exists but we reference "month" which doesn't exist
        // → empty string (not literal $<month>)
        let result = host_regex_replace_inner(hs(r"(?<year>\d{4})"), hs("2026"), hs("$<month>"));
        assert_eq!(result, "");
    }

    #[test]
    fn replace_no_match_returns_original() {
        let result = host_regex_replace_inner(hs(r"\d+"), hs("abcdef"), hs("X"));
        assert_eq!(result, "abcdef");
    }

    #[test]
    fn replace_empty_replacement_removes_match() {
        let result = host_regex_replace_inner(hs(r"\d+"), hs("abc123def"), hs(""));
        assert_eq!(result, "abcdef");
    }

    #[test]
    fn replace_trailing_dollar_is_literal() {
        let result = host_regex_replace_inner(hs(r"\d+"), hs("abc123def"), hs("X$"));
        assert_eq!(result, "abcX$def");
    }

    #[test]
    fn replace_unknown_dollar_pattern_is_literal_dollar() {
        // $x is not a recognized pattern → literal $ + x
        let result = host_regex_replace_inner(hs(r"\d+"), hs("abc123def"), hs("$x"));
        assert_eq!(result, "abc$xdef");
    }

    #[test]
    fn replace_all_replaces_all_matches() {
        let result = host_regex_replace_all_inner(hs(r"\d"), hs("a1b2c3"), hs("X"));
        assert_eq!(result, "aXbXcX");
    }

    #[test]
    fn replace_all_dollar_amp_each_match() {
        let result = host_regex_replace_all_inner(hs(r"\d"), hs("a1b2c3"), hs("[$&]"));
        assert_eq!(result, "a[1]b[2]c[3]");
    }

    #[test]
    fn replace_all_dollar_n_capture_groups() {
        // Swap two capture groups in all matches
        let result = host_regex_replace_all_inner(hs(r"(\d)(\w)"), hs("1a2b3c"), hs("$2$1"));
        assert_eq!(result, "a1b2c3");
    }

    #[test]
    fn replace_all_no_match_returns_original() {
        let result = host_regex_replace_all_inner(hs(r"\d"), hs("abc"), hs("X"));
        assert_eq!(result, "abc");
    }

    #[test]
    fn replace_preserves_unicode_around_match() {
        let result = host_regex_replace_inner(hs(r"\d+"), hs("café123naïve"), hs("X"));
        assert_eq!(result, "caféXnaïve");
    }

    #[test]
    fn replace_all_zero_width_match() {
        // Empty-match regex inserts replacement at each position
        let result = host_regex_replace_all_inner(hs(r""), hs("ab"), hs("X"));
        assert_eq!(result, "XaXbX");
    }
}
