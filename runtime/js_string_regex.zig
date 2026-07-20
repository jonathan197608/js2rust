//! JS String regex-dependent method implementations for Zig.
//! These functions rely on host_regex_* C ABI symbols provided by
//! js2rust-bridge (native_regex.rs) at link time.
//!
//! This module is only imported when the transpiled JS code uses
//! RegExp/regex features (needs_regex = true).

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsAny = @import("jsany.zig").JsAny;

// ── Host C ABI function declarations ──

extern fn host_regex_match(
    pattern_ptr: [*]const u8,
    pattern_len: usize,
    text_ptr: [*]const u8,
    text_len: usize,
    out_count: *usize,
) callconv(.c) extern struct { ptr: [*]const u8, len: isize };

extern fn host_regex_match_global(
    pattern_ptr: [*]const u8,
    pattern_len: usize,
    text_ptr: [*]const u8,
    text_len: usize,
    out_count: *usize,
) callconv(.c) extern struct { ptr: [*]const u8, len: isize };

extern fn host_regex_match_all(
    pattern_ptr: [*]const u8,
    pattern_len: usize,
    text_ptr: [*]const u8,
    text_len: usize,
    out_match_count: *usize,
    out_group_count: *usize,
) callconv(.c) extern struct { ptr: [*]const u8, len: isize };

extern fn host_regex_replace(
    pattern_ptr: [*]const u8,
    pattern_len: usize,
    text_ptr: [*]const u8,
    text_len: usize,
    replacement_ptr: [*]const u8,
    replacement_len: usize,
) callconv(.c) extern struct { ptr: [*]const u8, len: isize };

extern fn host_regex_replace_all(
    pattern_ptr: [*]const u8,
    pattern_len: usize,
    text_ptr: [*]const u8,
    text_len: usize,
    replacement_ptr: [*]const u8,
    replacement_len: usize,
) callconv(.c) extern struct { ptr: [*]const u8, len: isize };

/// Match string against regex via host_regex_match C ABI.
///
/// Returns null if no match, or an array of matched substrings.
/// Index 0 is the full match, indices 1+ are capture groups.
pub fn matchString(alloc: Allocator, s: []const u8, pattern: []const u8) !JsAny {
    var count: usize = 0;
    const result = host_regex_match(pattern.ptr, pattern.len, s.ptr, s.len, &count);
    if (count == 0) return JsAny.fromNull();

    const bytes = result.ptr[0..@intCast(result.len)];
    var arr = try JsAny.newArray(alloc);
    errdefer arr.array.deinit(alloc);

    var start: usize = 0;
    for (0..count) |_| {
        var end: usize = start;
        while (end < bytes.len and bytes[end] != 0) : (end += 1) {}
        try arr.array.append(alloc, JsAny.from(bytes[start..end]));
        start = end + 1;
    }

    return arr;
}

/// String.match() with /g flag — returns all matches (no capture groups).
/// Returns null if no match, or an array of matched substrings.
pub fn matchStringGlobal(alloc: Allocator, s: []const u8, pattern: []const u8) !JsAny {
    var count: usize = 0;
    const result = host_regex_match_global(pattern.ptr, pattern.len, s.ptr, s.len, &count);
    if (count == 0) return JsAny.fromNull();

    const bytes = result.ptr[0..@intCast(result.len)];
    var arr = try JsAny.newArray(alloc);
    errdefer arr.array.deinit(alloc);

    var start: usize = 0;
    for (0..count) |_| {
        var end: usize = start;
        while (end < bytes.len and bytes[end] != 0) : (end += 1) {}
        try arr.array.append(alloc, JsAny.from(bytes[start..end]));
        start = end + 1;
    }

    return arr;
}

/// String.matchAll(regex) — returns array of match arrays (with capture groups).
/// Each match array: [0] = full match, [1..] = capture groups.
/// Returns empty array if no match (JS matchAll never returns null).
pub fn matchAllString(alloc: Allocator, s: []const u8, pattern: []const u8) !JsAny {
    var match_count: usize = 0;
    var group_count: usize = 0;
    const result = host_regex_match_all(pattern.ptr, pattern.len, s.ptr, s.len, &match_count, &group_count);

    // matchAll always returns an iterator (empty if no matches)
    var outer_arr = try JsAny.newArray(alloc);
    errdefer outer_arr.array.deinit(alloc);

    if (match_count == 0 or group_count == 0) return outer_arr;

    const bytes = result.ptr[0..@intCast(result.len)];

    // Parse NUL-separated groups into match arrays
    var pos: usize = 0;
    for (0..match_count) |_| {
        var inner_arr = try JsAny.newArray(alloc);
        for (0..group_count) |_| {
            var end: usize = pos;
            while (end < bytes.len and bytes[end] != 0) : (end += 1) {}
            if (pos < bytes.len) {
                try inner_arr.array.append(alloc, JsAny.from(bytes[pos..end]));
            } else {
                try inner_arr.array.append(alloc, JsAny.from(""));
            }
            pos = end + 1;
        }
        try outer_arr.array.append(alloc, inner_arr);
    }

    return outer_arr;
}

/// String.replace(regex, replacement) — replaces the first match.
/// Returns the result string (always non-null in JS).
pub fn replaceRegex(alloc: Allocator, s: []const u8, pattern: []const u8, replacement: []const u8) ![]const u8 {
    const result = host_regex_replace(pattern.ptr, pattern.len, s.ptr, s.len, replacement.ptr, replacement.len);
    if (result.len == 0 and s.len == 0) {
        // Both input and result are empty — nothing to dupe
        return alloc.dupe(u8, "");
    }
    const bytes = result.ptr[0..@intCast(result.len)];
    return alloc.dupe(u8, bytes);
}

/// String.replaceAll(regex, replacement) — replaces all matches.
/// Returns the result string (always non-null in JS).
pub fn replaceAllRegex(alloc: Allocator, s: []const u8, pattern: []const u8, replacement: []const u8) ![]const u8 {
    const result = host_regex_replace_all(pattern.ptr, pattern.len, s.ptr, s.len, replacement.ptr, replacement.len);
    if (result.len == 0 and s.len == 0) {
        // Both input and result are empty — nothing to dupe
        return alloc.dupe(u8, "");
    }
    const bytes = result.ptr[0..@intCast(result.len)];
    return alloc.dupe(u8, bytes);
}
