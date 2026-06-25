//! JS String method implementations for Zig.
//! All allocating functions take `alloc: std.mem.Allocator` as first parameter.

const std = @import("std");
const Allocator = std.mem.Allocator;

/// Convert string to uppercase. Returns newly allocated string.
pub fn toUpper(alloc: Allocator, s: []const u8) ![]const u8 {
    const result = try alloc.alloc(u8, s.len);
    for (s, 0..) |c, i| {
        result[i] = std.ascii.toUpper(c);
    }
    return result;
}

/// Convert string to lowercase. Returns newly allocated string.
pub fn toLower(alloc: Allocator, s: []const u8) ![]const u8 {
    const result = try alloc.alloc(u8, s.len);
    for (s, 0..) |c, i| {
        result[i] = std.ascii.toLower(c);
    }
    return result;
}

/// Get character at index, returned as a 1-char string.
pub fn charAt(alloc: Allocator, s: []const u8, idx: i64) ![]const u8 {
    const uidx: usize = @intCast(idx);
    if (uidx >= s.len) return &[0]u8{};
    const result = try alloc.alloc(u8, 1);
    result[0] = s[uidx];
    return result;
}

/// Get character at index (supports negative indices), returned as a 1-char string.
/// Negative indices count from the end: at(-1) returns the last character.
pub fn at(alloc: Allocator, s: []const u8, idx: i64) ![]const u8 {
    const adjusted_idx: isize = if (idx < 0) @as(isize, @intCast(s.len)) + @as(isize, idx) else idx;
    if (adjusted_idx < 0 or adjusted_idx >= s.len) return &[0]u8{};
    const uidx: usize = @intCast(adjusted_idx);
    const result = try alloc.alloc(u8, 1);
    result[0] = s[uidx];
    return result;
}

/// Get UTF-16 code unit at index (JS charCodeAt behavior).
/// Returns the i-th UTF-16 code unit (0-65535).
/// If idx is out of bounds, returns 0 (JS returns NaN, but we return 0 for type simplicity).
pub fn charCodeAt(s: []const u8, idx: i64) u16 {
    const target: usize = @intCast(@max(0, idx));
    var utf16_idx: usize = 0;
    var i: usize = 0;

    while (i < s.len) {
        // Decode UTF-8 code point
        const c = s[i];
        var code_point: u32 = 0;
        var seq_len: u8 = 1;

        if (c & 0x80 == 0) {
            // 1-byte: 0xxxxxxx (ASCII)
            code_point = c;
            seq_len = 1;
        } else if (c & 0xE0 == 0xC0) {
            // 2-byte: 110xxxxx 10xxxxxx
            code_point = (@as(u32, c & 0x1F) << 6) | @as(u32, s[i + 1] & 0x3F);
            seq_len = 2;
        } else if (c & 0xF0 == 0xE0) {
            // 3-byte: 1110xxxx 10xxxxxx 10xxxxxx
            code_point = (@as(u32, c & 0x0F) << 12) | (@as(u32, s[i + 1] & 0x3F) << 6) | @as(u32, s[i + 2] & 0x3F);
            seq_len = 3;
        } else if (c & 0xF8 == 0xF0) {
            // 4-byte: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
            code_point = (@as(u32, c & 0x07) << 18) | (@as(u32, s[i + 1] & 0x3F) << 12) | (@as(u32, s[i + 2] & 0x3F) << 6) | @as(u32, s[i + 3] & 0x3F);
            seq_len = 4;
        } else {
            // Invalid UTF-8 byte, skip
            i += 1;
            continue;
        }

        // Check if this is the target UTF-16 index
        if (code_point <= 0xFFFF) {
            // BMP character: 1 UTF-16 code unit
            if (utf16_idx == target) {
                return @intCast(code_point);
            }
            utf16_idx += 1;
        } else {
            // Supplementary plane character: 2 UTF-16 code units (surrogate pair)
            const high: u16 = @intCast(0xD800 + ((code_point - 0x10000) >> 10));
            const low: u16 = @intCast(0xDC00 + ((code_point - 0x10000) & 0x3FF));
            if (utf16_idx == target) {
                return high;
            }
            if (utf16_idx + 1 == target) {
                return low;
            }
            utf16_idx += 2;
        }

        i += seq_len;
    }

    return 0; // Out of bounds
}

/// Get Unicode code point at index (JS codePointAt behavior).
/// Returns the Unicode code point (U+0000 to U+10FFFF) as i64.
/// If idx is out of bounds, returns 0 (JS returns undefined, but we return 0 for type simplicity).
/// Unlike charCodeAt(), this correctly handles surrogate pairs:
/// - If the index points to a high surrogate (0xD800-0xDBFF) and the next code unit
///   is a low surrogate (0xDC00-0xDFFF), decodes the pair and returns the full code point.
pub fn codePointAt(s: []const u8, idx: i64) i64 {
    const target: usize = @intCast(@max(0, idx));
    var utf16_idx: usize = 0;
    var i: usize = 0;

    while (i < s.len) {
        // Decode UTF-8 code point
        const c = s[i];
        var code_point: u32 = 0;
        var seq_len: u8 = 1;

        if (c & 0x80 == 0) {
            code_point = c;
            seq_len = 1;
        } else if (c & 0xE0 == 0xC0) {
            code_point = (@as(u32, c & 0x1F) << 6) | @as(u32, s[i + 1] & 0x3F);
            seq_len = 2;
        } else if (c & 0xF0 == 0xE0) {
            code_point = (@as(u32, c & 0x0F) << 12) | (@as(u32, s[i + 1] & 0x3F) << 6) | @as(u32, s[i + 2] & 0x3F);
            seq_len = 3;
        } else if (c & 0xF8 == 0xF0) {
            code_point = (@as(u32, c & 0x07) << 18) | (@as(u32, s[i + 1] & 0x3F) << 12) | (@as(u32, s[i + 2] & 0x3F) << 6) | @as(u32, s[i + 3] & 0x3F);
            seq_len = 4;
        } else {
            // Invalid UTF-8 byte, skip
            i += 1;
            continue;
        }

        // Check if this is the target UTF-16 index
        if (code_point <= 0xFFFF) {
            // BMP character: 1 UTF-16 code unit
            if (utf16_idx == target) {
                return @intCast(code_point);
            }
            utf16_idx += 1;
        } else {
            // Supplementary plane character: 2 UTF-16 code units (surrogate pair)
            if (utf16_idx == target) {
                // Return the full code point (not just the high surrogate)
                return @intCast(code_point);
            }
            // Skip the low surrogate (it's part of the same code point)
            utf16_idx += 2;
        }

        i += seq_len;
    }

    return 0; // Out of bounds
}

/// Concatenate two strings. Returns newly allocated string.
pub fn concat(alloc: Allocator, a: []const u8, b: []const u8) ![]const u8 {
    const result = try alloc.alloc(u8, a.len + b.len);
    @memcpy(result[0..a.len], a);
    @memcpy(result[a.len..], b);
    return result;
}

/// Check if haystack contains needle.
pub fn includes(haystack: []const u8, needle: []const u8) bool {
    return std.mem.indexOf(u8, haystack, needle) != null;
}

/// Find index of needle in haystack, or -1 if not found.
pub fn indexOf(haystack: []const u8, needle: []const u8) i64 {
    if (std.mem.indexOf(u8, haystack, needle)) |pos| {
        return @intCast(pos);
    }
    return -1;
}

/// Check if s starts with prefix.
pub fn startsWith(s: []const u8, prefix: []const u8) bool {
    return std.mem.startsWith(u8, s, prefix);
}

/// Check if s ends with suffix.
pub fn endsWith(s: []const u8, suffix: []const u8) bool {
    return std.mem.endsWith(u8, s, suffix);
}

/// Extract substring from start (inclusive) to end (exclusive).
/// Negative indices count from the end. Returns borrowed slice (no allocation).
pub fn slice(s: []const u8, start: i64, end: i64) []const u8 {
    const len: i64 = @intCast(s.len);
    var st: i64 = start;
    var en: i64 = end;

    if (st < 0) st = @max(0, len + st);
    if (en < 0) en = @max(0, len + en);

    st = @min(@max(0, st), len);
    en = @min(@max(0, en), len);
    if (st >= en) return &[0]u8{};

    return s[@intCast(st)..@intCast(en)];
}

/// Extract substring from startIndex to endIndex (JS substring semantics).
/// - If either arg is negative or NaN, treat as 0.
/// - If either arg > length, treat as length.
/// - If startIndex > endIndex, swap them.
/// Returns borrowed slice (no allocation).
pub fn substring(s: []const u8, start: i64, end: i64) []const u8 {
    const len: i64 = @intCast(s.len);
    var st: i64 = start;
    var en: i64 = end;

    // Clamp negative → 0
    if (st < 0) st = 0;
    if (en < 0) en = 0;

    // Clamp > length → length
    st = @min(st, len);
    en = @min(en, len);

    // Swap if start > end
    if (st > en) {
        const tmp = st;
        st = en;
        en = tmp;
    }

    return s[@intCast(st)..@intCast(en)];
}

/// Split string by separator. Returns newly allocated array of strings.
pub fn split(alloc: Allocator, s: []const u8, sep: []const u8) ![][]const u8 {
    var parts = std.ArrayList([]const u8).empty;
    errdefer parts.deinit(alloc);

    var remaining = s;
    while (std.mem.indexOf(u8, remaining, sep)) |pos| {
        try parts.append(alloc, remaining[0..pos]);
        remaining = remaining[pos + sep.len ..];
    }
    try parts.append(alloc, remaining);

    return parts.toOwnedSlice(alloc);
}

/// Replace all occurrences of old with new. Returns newly allocated string.
pub fn replace(alloc: Allocator, s: []const u8, old: []const u8, new: []const u8) ![]const u8 {
    return std.mem.replaceOwned(u8, alloc, s, old, new);
}

/// Trim whitespace from both ends. Returns borrowed slice.
pub fn trim(s: []const u8) []const u8 {
    return std.mem.trim(u8, s, &std.ascii.whitespace);
}

/// Repeat string n times. Returns newly allocated string.
pub fn repeat(alloc: Allocator, s: []const u8, n: i64) ![]const u8 {
    const count: usize = @intCast(@max(0, n));
    const result = try alloc.alloc(u8, s.len * count);
    var i: usize = 0;
    while (i < count) : (i += 1) {
        @memcpy(result[i * s.len .. (i + 1) * s.len], s);
    }
    return result;
}

/// Pad the start of a string to reach target_len using pad_str repeated.
pub fn padStart(alloc: Allocator, s: []const u8, target_len: i64, pad_str: []const u8) ![]const u8 {
    const target: usize = @intCast(@max(0, target_len));
    if (s.len >= target or pad_str.len == 0) return try alloc.dupe(u8, s);
    const pad_needed = target - s.len;
    const result = try alloc.alloc(u8, target);
    var written: usize = 0;
    while (written < pad_needed) {
        const rem = pad_needed - written;
        const chunk = @min(rem, pad_str.len);
        @memcpy(result[written..][0..chunk], pad_str[0..chunk]);
        written += chunk;
    }
    @memcpy(result[pad_needed..], s);
    return result;
}

/// Pad the end of a string to reach target_len using pad_str repeated.
pub fn padEnd(alloc: Allocator, s: []const u8, target_len: i64, pad_str: []const u8) ![]const u8 {
    const target: usize = @intCast(@max(0, target_len));
    if (s.len >= target or pad_str.len == 0) return try alloc.dupe(u8, s);
    const result = try alloc.alloc(u8, target);
    @memcpy(result[0..s.len], s);
    var written: usize = s.len;
    while (written < target) {
        const rem = target - written;
        const chunk = @min(rem, pad_str.len);
        @memcpy(result[written..][0..chunk], pad_str[0..chunk]);
        written += chunk;
    }
    return result;
}

test "toUpper" {
    const result = try toUpper(std.testing.allocator, "hello");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("HELLO", result);
}

test "toLower" {
    const result = try toLower(std.testing.allocator, "HELLO");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello", result);
}

test "charAt" {
    const result = try charAt(std.testing.allocator, "abc", 1);
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("b", result);
}

test "concat" {
    const result = try concat(std.testing.allocator, "hello", " world");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello world", result);
}

test "includes" {
    try std.testing.expect(includes("hello world", "world"));
    try std.testing.expect(!includes("hello world", "xyz"));
}

test "indexOf" {
    try std.testing.expectEqual(@as(i64, 6), indexOf("hello world", "world"));
    try std.testing.expectEqual(@as(i64, -1), indexOf("hello world", "xyz"));
}

test "startsWith" {
    try std.testing.expect(startsWith("hello", "hel"));
    try std.testing.expect(!startsWith("hello", "xyz"));
}

test "endsWith" {
    try std.testing.expect(endsWith("hello", "llo"));
    try std.testing.expect(!endsWith("hello", "hel"));
}

test "slice" {
    try std.testing.expectEqualStrings("ell", slice("hello", 1, 4));
    try std.testing.expectEqualStrings("lo", slice("hello", -2, 5));
}

test "split" {
    const alloc = std.testing.allocator;
    const result = try split(alloc, "a,b,c", ",");
    defer alloc.free(result);
    try std.testing.expectEqual(@as(usize, 3), result.len);
    try std.testing.expectEqualStrings("a", result[0]);
    try std.testing.expectEqualStrings("c", result[2]);
}

test "replace" {
    const result = try replace(std.testing.allocator, "hello world", "world", "zig");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello zig", result);
}

test "trim" {
    try std.testing.expectEqualStrings("hello", trim("  hello  "));
}

test "repeat" {
    const result = try repeat(std.testing.allocator, "ab", 3);
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("ababab", result);
}

test "charCodeAt ASCII" {
    // ASCII characters
    try std.testing.expectEqual(@as(u16, 72), charCodeAt("Hello", 0)); // 'H'
    try std.testing.expectEqual(@as(u16, 101), charCodeAt("Hello", 1)); // 'e'
    try std.testing.expectEqual(@as(u16, 108), charCodeAt("Hello", 2)); // 'l'
    try std.testing.expectEqual(@as(u16, 108), charCodeAt("Hello", 3)); // Second 'l'
    try std.testing.expectEqual(@as(u16, 111), charCodeAt("Hello", 4)); // 'o'
    try std.testing.expectEqual(@as(u16, 0), charCodeAt("Hello", 10)); // Out of bounds
}

test "charCodeAt UTF-8" {
    // Multi-byte UTF-8 characters
    // 'café' - 'c'=99, 'a'=97, 'f'=102, 'é'=U+00E9=233
    try std.testing.expectEqual(@as(u16, 99), charCodeAt("café", 0));
    try std.testing.expectEqual(@as(u16, 97), charCodeAt("café", 1));
    try std.testing.expectEqual(@as(u16, 233), charCodeAt("café", 3)); // 'é' (U+00E9)
}

test "charCodeAt surrogate pair" {
    // Supplementary plane character (U+1F600 = 😀)
    // UTF-16: surrogate pair 0xD83D 0xDE00
    const emoji = "😀";
    const high = charCodeAt(emoji, 0);
    const low = charCodeAt(emoji, 1);
    try std.testing.expectEqual(@as(u16, 0xD83D), high); // High surrogate
    try std.testing.expectEqual(@as(u16, 0xDE00), low); // Low surrogate
}

test "padStart" {
    const result = try padStart(std.testing.allocator, "42", 5, "0");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("00042", result);
}

test "padStart no-op" {
    const result = try padStart(std.testing.allocator, "hello", 3, ".");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello", result);
}

test "padEnd" {
    const result = try padEnd(std.testing.allocator, "hello", 10, ".");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello.....", result);
}

test "padEnd no-op" {
    const result = try padEnd(std.testing.allocator, "abc", 3, ".");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("abc", result);
}

/// Locale-sensitive string comparison (simplified: byte-wise comparison).
/// Returns -1 if self < other, 0 if equal, 1 if self > other.
/// Note: This is a simplified implementation that uses byte-wise comparison.
/// For proper locale-sensitive comparison, an ICU library would be needed.
pub fn localeCompare(self: []const u8, other: []const u8) i64 {
    return switch (std.mem.order(u8, self, other)) {
        .lt => -1,
        .eq => 0,
        .gt => 1,
    };
}

/// Normalize Unicode string (stub: returns a copy of the input).
/// In a full implementation, this would apply Unicode normalization form (NFC, NFD, NFKC, NFKD).
/// Currently this is a pass-through stub.
pub fn normalize(alloc: Allocator, s: []const u8, form: []const u8) ![]const u8 {
    _ = form; // Ignore normalization form for now
    return try alloc.dupe(u8, s);
}

/// Convert string to locale-specific uppercase (simplified: uses ASCII toUpper).
/// Note: This is a simplified implementation. For proper locale-specific casing,
/// an ICU library would be needed.
pub fn toLocaleUpper(alloc: Allocator, s: []const u8) ![]const u8 {
    return toUpper(alloc, s);
}

/// Convert string to locale-specific lowercase (simplified: uses ASCII toLower).
/// Note: This is a simplified implementation. For proper locale-specific casing,
/// an ICU library would be needed.
pub fn toLocaleLower(alloc: Allocator, s: []const u8) ![]const u8 {
    return toLower(alloc, s);
}

/// Replace all occurrences of old with new. Returns newly allocated string.
pub fn replaceAll(alloc: Allocator, s: []const u8, old: []const u8, new: []const u8) ![]const u8 {
    return std.mem.replaceOwned(u8, alloc, s, old, new);
}

/// Create string from character code(s). Takes UTF-16 code units and returns a UTF-8 string.
pub fn fromCharCode(alloc: Allocator, codes: []const i64) ![]const u8 {
    // Calculate required buffer size
    var buf_size: usize = 0;
    for (codes) |code| {
        const c: u32 = @intCast(@max(0, @min(code, 0xFFFF)));
        if (c <= 0x7F) {
            buf_size += 1;
        } else if (c <= 0x7FF) {
            buf_size += 2;
        } else {
            buf_size += 3;
        }
    }
    
    const result = try alloc.alloc(u8, buf_size);
    var pos: usize = 0;
    for (codes) |code| {
        const c: u32 = @intCast(@max(0, @min(code, 0xFFFF)));
        if (c <= 0x7F) {
            result[pos] = @intCast(c);
            pos += 1;
        } else if (c <= 0x7FF) {
            result[pos] = @intCast(0xC0 | (c >> 6));
            result[pos + 1] = @intCast(0x80 | (c & 0x3F));
            pos += 2;
        } else {
            result[pos] = @intCast(0xE0 | (c >> 12));
            result[pos + 1] = @intCast(0x80 | ((c >> 6) & 0x3F));
            result[pos + 2] = @intCast(0x80 | (c & 0x3F));
            pos += 3;
        }
    }
    return result;
}

/// Create string from Unicode code point(s). Takes Unicode code points (U+0000 to U+10FFFF).
pub fn fromCodePoint(alloc: Allocator, code_points: []const i64) ![]const u8 {
    // Calculate required buffer size
    var buf_size: usize = 0;
    for (code_points) |cp| {
        const c: u32 = @intCast(@max(0, cp));
        if (c <= 0x7F) {
            buf_size += 1;
        } else if (c <= 0x7FF) {
            buf_size += 2;
        } else if (c <= 0xFFFF) {
            buf_size += 3;
        } else {
            buf_size += 4;
        }
    }
    
    const result = try alloc.alloc(u8, buf_size);
    var pos: usize = 0;
    for (code_points) |cp| {
        const c: u32 = @intCast(@max(0, cp));
        if (c <= 0x7F) {
            result[pos] = @intCast(c);
            pos += 1;
        } else if (c <= 0x7FF) {
            result[pos] = @intCast(0xC0 | (c >> 6));
            result[pos + 1] = @intCast(0x80 | (c & 0x3F));
            pos += 2;
        } else if (c <= 0xFFFF) {
            result[pos] = @intCast(0xE0 | (c >> 12));
            result[pos + 1] = @intCast(0x80 | ((c >> 6) & 0x3F));
            result[pos + 2] = @intCast(0x80 | (c & 0x3F));
            pos += 3;
        } else {
            result[pos] = @intCast(0xF0 | (c >> 18));
            result[pos + 1] = @intCast(0x80 | ((c >> 12) & 0x3F));
            result[pos + 2] = @intCast(0x80 | ((c >> 6) & 0x3F));
            result[pos + 3] = @intCast(0x80 | (c & 0x3F));
            pos += 4;
        }
    }
    return result;
}

/// Match string against regex (stub: always returns null).
/// Requires regex engine support.
pub fn matchString(s: []const u8, regex: []const u8) !?[][]const u8 {
    _ = s;
    _ = regex;
    return null;
}

/// Search string for regex (stub: always returns -1).
/// Requires regex engine support.
pub fn searchString(s: []const u8, regex: []const u8) i64 {
    _ = s;
    _ = regex;
    return -1;
}

test "localeCompare" {
    try std.testing.expectEqual(@as(i64, -1), localeCompare("apple", "banana"));
    try std.testing.expectEqual(@as(i64, 0), localeCompare("apple", "apple"));
    try std.testing.expectEqual(@as(i64, 1), localeCompare("banana", "apple"));
}

test "normalize stub" {
    const result = try normalize(std.testing.allocator, "café", "NFC");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("café", result);
}

test "toLocaleUpper" {
    const result = try toLocaleUpper(std.testing.allocator, "hello");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("HELLO", result);
}

test "toLocaleLower" {
    const result = try toLocaleLower(std.testing.allocator, "HELLO");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello", result);
}

test "replaceAll" {
    const result = try replaceAll(std.testing.allocator, "hello world world", "world", "zig");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello zig zig", result);
}

test "fromCharCode" {
    const result = try fromCharCode(std.testing.allocator, &[_]i64{ 72, 101, 108, 108, 111 });
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("Hello", result);
}

test "fromCodePoint" {
    const result = try fromCodePoint(std.testing.allocator, &[_]i64{ 72, 101, 108, 108, 111 });
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("Hello", result);
}
