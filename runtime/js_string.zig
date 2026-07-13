//! JS String method implementations for Zig.
//! All allocating functions take `alloc: std.mem.Allocator` as first parameter.

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsAny = @import("jsany.zig").JsAny;

// ── Host C ABI function declarations ──
// Regex-dependent extern declarations have been moved to js_string_regex.zig.
// They are only needed when needs_regex = true.

// ── Internal UTF-8/UTF-16 helpers ──

/// Decode a single UTF-8 code point starting at position i.
/// Returns the decoded code point and the byte sequence length, or null if invalid.
fn decodeUtf8CodePoint(s: []const u8, i: usize) ?struct { code_point: u32, len: u8 } {
    if (i >= s.len) return null;
    const c = s[i];
    var code_point: u32 = 0;
    var seq_len: u8 = 1;

    if (c & 0x80 == 0) {
        // 1-byte: 0xxxxxxx (ASCII)
        code_point = c;
        seq_len = 1;
    } else if (c & 0xE0 == 0xC0) {
        // 2-byte: 110xxxxx 10xxxxxx
        if (i + 1 >= s.len) return null;
        code_point = (@as(u32, c & 0x1F) << 6) | @as(u32, s[i + 1] & 0x3F);
        seq_len = 2;
    } else if (c & 0xF0 == 0xE0) {
        // 3-byte: 1110xxxx 10xxxxxx 10xxxxxx
        if (i + 2 >= s.len) return null;
        code_point = (@as(u32, c & 0x0F) << 12) | (@as(u32, s[i + 1] & 0x3F) << 6) | @as(u32, s[i + 2] & 0x3F);
        seq_len = 3;
    } else if (c & 0xF8 == 0xF0) {
        // 4-byte: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
        if (i + 3 >= s.len) return null;
        code_point = (@as(u32, c & 0x07) << 18) | (@as(u32, s[i + 1] & 0x3F) << 12) | (@as(u32, s[i + 2] & 0x3F) << 6) | @as(u32, s[i + 3] & 0x3F);
        seq_len = 4;
    } else {
        return null;
    }

    return .{ .code_point = code_point, .len = seq_len };
}

/// Number of UTF-16 code units needed to represent a Unicode code point.
fn utf16CodeUnitCount(code_point: u32) usize {
    return if (code_point <= 0xFFFF) 1 else 2;
}

// ── Public UTF-16 helper functions ──

/// Count the number of UTF-16 code units in a UTF-8 string.
/// JS string.length returns UTF-16 code unit count, not byte count.
pub fn utf16Len(s: []const u8) usize {
    var len: usize = 0;
    var i: usize = 0;
    while (i < s.len) {
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };
        len += utf16CodeUnitCount(decoded.code_point);
        i += decoded.len;
    }
    return len;
}

/// Convert a UTF-16 code unit index to a byte offset in the UTF-8 string.
/// Returns null if the index is out of bounds (>= utf16Len).
/// Passing idx == utf16Len returns s.len (one-past-end, for slicing).
/// For supplementary characters, the low surrogate index maps to the same
/// byte offset as the high surrogate (both are part of the same code point).
pub fn utf16IndexToByteOffset(s: []const u8, idx: usize) ?usize {
    var utf16_idx: usize = 0;
    var i: usize = 0;
    while (i < s.len) {
        const code_point_start = i;
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };
        const cu_count = utf16CodeUnitCount(decoded.code_point);
        // Check if idx falls within this code point's UTF-16 code units
        if (idx >= utf16_idx and idx < utf16_idx + cu_count) {
            return code_point_start; // Return byte offset of the code point
        }
        utf16_idx += cu_count;
        i += decoded.len;
    }
    // idx is exactly at the end (one-past-end)
    if (idx == utf16_idx) return i;
    return null;
}

/// Convert a byte offset in a UTF-8 string to the corresponding UTF-16 code unit index.
pub fn byteOffsetToUtf16Index(s: []const u8, byte_off: usize) usize {
    var utf16_idx: usize = 0;
    var i: usize = 0;
    while (i < byte_off and i < s.len) {
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };
        utf16_idx += utf16CodeUnitCount(decoded.code_point);
        i += decoded.len;
    }
    return utf16_idx;
}

/// Return the byte slice containing exactly the first `n` UTF-16 code units of `s`.
/// Used by padStart/padEnd for partial padding truncation.
pub fn firstUtf16CodeUnits(s: []const u8, n: usize) []const u8 {
    var utf16_idx: usize = 0;
    var i: usize = 0;
    while (i < s.len and utf16_idx < n) {
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };
        const cu_count = utf16CodeUnitCount(decoded.code_point);
        if (utf16_idx + cu_count > n) break; // would overshoot
        utf16_idx += cu_count;
        i += decoded.len;
    }
    return s[0..i];
}

/// Encode a single UTF-16 code unit as a UTF-8 string.
/// For BMP characters this produces standard UTF-8.
/// For surrogate code points (0xD800-0xDFFF) this produces CESU-8 (3-byte encoding),
/// which matches JS semantics where charAt can return a lone surrogate.
fn encodeCodeUnit(alloc: Allocator, cu: u16) ![]const u8 {
    const cp: u32 = cu;
    if (cp <= 0x7F) {
        const result = try alloc.alloc(u8, 1);
        result[0] = @intCast(cp);
        return result;
    } else if (cp <= 0x7FF) {
        const result = try alloc.alloc(u8, 2);
        result[0] = @intCast(0xC0 | (cp >> 6));
        result[1] = @intCast(0x80 | (cp & 0x3F));
        return result;
    } else {
        // BMP including surrogates → 3-byte encoding (CESU-8 for surrogates)
        const result = try alloc.alloc(u8, 3);
        result[0] = @intCast(0xE0 | (cp >> 12));
        result[1] = @intCast(0x80 | ((cp >> 6) & 0x3F));
        result[2] = @intCast(0x80 | (cp & 0x3F));
        return result;
    }
}

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

/// Get character at index, returned as a string.
/// Uses UTF-16 code unit indexing (JS semantics).
/// For supplementary characters, charAt at the high surrogate index returns the
/// high surrogate as a CESU-8 string; charAt at the low surrogate index returns
/// the low surrogate as a CESU-8 string.
pub fn charAt(alloc: Allocator, s: []const u8, idx: i64) ![]const u8 {
    if (idx < 0) return &[0]u8{};
    const target: usize = @intCast(idx);
    var utf16_idx: usize = 0;
    var i: usize = 0;
    while (i < s.len) {
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };
        if (decoded.code_point <= 0xFFFF) {
            // BMP character: 1 UTF-16 code unit
            if (utf16_idx == target) {
                return encodeCodeUnit(alloc, @intCast(decoded.code_point));
            }
            utf16_idx += 1;
        } else {
            // Supplementary character: 2 UTF-16 code units (surrogate pair)
            const high: u16 = @intCast(0xD800 + ((decoded.code_point - 0x10000) >> 10));
            const low: u16 = @intCast(0xDC00 + ((decoded.code_point - 0x10000) & 0x3FF));
            if (utf16_idx == target) {
                return encodeCodeUnit(alloc, high);
            }
            if (utf16_idx + 1 == target) {
                return encodeCodeUnit(alloc, low);
            }
            utf16_idx += 2;
        }
        i += decoded.len;
    }
    return &[0]u8{}; // Out of bounds
}

/// Get character at index (supports negative indices), returned as a string.
/// Negative indices count from the end: at(-1) returns the last character.
/// Uses UTF-16 code unit indexing (JS semantics).
pub fn at(alloc: Allocator, s: []const u8, idx: i64) ![]const u8 {
    const len: i64 = @intCast(utf16Len(s));
    const adjusted_idx: i64 = if (idx < 0) len + idx else idx;
    if (adjusted_idx < 0 or adjusted_idx >= len) return &[0]u8{};
    return charAt(alloc, s, adjusted_idx);
}

/// Get UTF-16 code unit at index (JS charCodeAt behavior).
/// Returns the i-th UTF-16 code unit (0-65535).
/// If idx is out of bounds, returns 0 (JS returns NaN, but we return 0 for type simplicity).
pub fn charCodeAt(s: []const u8, idx: i64) u16 {
    const target: usize = @intCast(@max(0, idx));
    var utf16_idx: usize = 0;
    var i: usize = 0;

    while (i < s.len) {
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };

        if (decoded.code_point <= 0xFFFF) {
            // BMP character: 1 UTF-16 code unit
            if (utf16_idx == target) {
                return @intCast(decoded.code_point);
            }
            utf16_idx += 1;
        } else {
            // Supplementary plane character: 2 UTF-16 code units (surrogate pair)
            const high: u16 = @intCast(0xD800 + ((decoded.code_point - 0x10000) >> 10));
            const low: u16 = @intCast(0xDC00 + ((decoded.code_point - 0x10000) & 0x3FF));
            if (utf16_idx == target) {
                return high;
            }
            if (utf16_idx + 1 == target) {
                return low;
            }
            utf16_idx += 2;
        }

        i += decoded.len;
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
        const decoded = decodeUtf8CodePoint(s, i) orelse {
            i += 1;
            continue;
        };

        if (decoded.code_point <= 0xFFFF) {
            // BMP character: 1 UTF-16 code unit
            if (utf16_idx == target) {
                return @intCast(decoded.code_point);
            }
            utf16_idx += 1;
        } else {
            // Supplementary plane character: 2 UTF-16 code units (surrogate pair)
            if (utf16_idx == target) {
                // Return the full code point (not just the high surrogate)
                return @intCast(decoded.code_point);
            }
            // Skip the low surrogate (it's part of the same code point)
            utf16_idx += 2;
        }

        i += decoded.len;
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
/// Returns UTF-16 code unit index (JS semantics).
pub fn indexOf(haystack: []const u8, needle: []const u8) i64 {
    if (std.mem.indexOf(u8, haystack, needle)) |pos| {
        return @intCast(byteOffsetToUtf16Index(haystack, pos));
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
/// Uses UTF-16 code unit indexing (JS semantics).
pub fn slice(s: []const u8, start: i64, end: i64) []const u8 {
    const len: i64 = @intCast(utf16Len(s));
    var st: i64 = start;
    var en: i64 = end;

    if (st < 0) st = @max(0, len + st);
    if (en < 0) en = @max(0, len + en);

    st = @min(@max(0, st), len);
    en = @min(@max(0, en), len);
    if (st >= en) return &[0]u8{};

    const byte_start = utf16IndexToByteOffset(s, @intCast(st)) orelse s.len;
    const byte_end = utf16IndexToByteOffset(s, @intCast(en)) orelse s.len;
    return s[byte_start..byte_end];
}

/// Extract substring from startIndex to endIndex (JS substring semantics).
/// - If either arg is negative or NaN, treat as 0.
/// - If either arg > length, treat as length.
/// - If startIndex > endIndex, swap them.
/// Returns borrowed slice (no allocation).
/// Uses UTF-16 code unit indexing (JS semantics).
pub fn substring(s: []const u8, start: i64, end: i64) []const u8 {
    const len: i64 = @intCast(utf16Len(s));
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

    const byte_start = utf16IndexToByteOffset(s, @intCast(st)) orelse s.len;
    const byte_end = utf16IndexToByteOffset(s, @intCast(en)) orelse s.len;
    return s[byte_start..byte_end];
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

/// Trim whitespace from the start (left) of a string.
pub fn trimStart(s: []const u8) []const u8 {
    return std.mem.trimLeft(u8, s, &std.ascii.whitespace);
}

/// Trim whitespace from the end (right) of a string.
pub fn trimEnd(s: []const u8) []const u8 {
    return std.mem.trimRight(u8, s, &std.ascii.whitespace);
}

/// Find the last index of needle in haystack.
/// Returns UTF-16 code unit index as i64, or -1 if not found.
pub fn lastIndexOf(haystack: []const u8, needle: []const u8) i64 {
    const haystack_utf16_len = utf16Len(haystack);
    if (needle.len == 0) return @intCast(haystack_utf16_len);
    if (needle.len > haystack.len) return -1;
    var i: i64 = @intCast(haystack.len - needle.len);
    while (i >= 0) : (i -= 1) {
        const start: usize = @intCast(i);
        if (std.mem.eql(u8, haystack[start .. start + needle.len], needle)) {
            return @intCast(byteOffsetToUtf16Index(haystack, start));
        }
    }
    return -1;
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
/// Uses UTF-16 code unit indexing (JS semantics): target_len is in UTF-16 code units.
pub fn padStart(alloc: Allocator, s: []const u8, target_len: i64, pad_str: []const u8) ![]const u8 {
    const s_utf16_len: usize = utf16Len(s);
    const pad_utf16_len: usize = utf16Len(pad_str);
    const target: usize = @intCast(@max(0, target_len));
    if (s_utf16_len >= target or pad_utf16_len == 0) return try alloc.dupe(u8, s);

    const pad_needed: usize = target - s_utf16_len; // in UTF-16 code units
    // Calculate how many full repetitions + partial prefix of pad_str we need
    const full_reps = pad_needed / pad_utf16_len;
    const partial_units = pad_needed % pad_utf16_len;

    // Byte size of the partial prefix of pad_str (first `partial_units` UTF-16 code units)
    const partial_slice = if (partial_units > 0) firstUtf16CodeUnits(pad_str, partial_units) else &[0]u8{};

    const total_pad_bytes = full_reps * pad_str.len + partial_slice.len;
    const result = try alloc.alloc(u8, total_pad_bytes + s.len);
    var pos: usize = 0;

    // Write full repetitions of pad_str
    for (0..full_reps) |_| {
        @memcpy(result[pos..][0..pad_str.len], pad_str);
        pos += pad_str.len;
    }

    // Write partial prefix
    if (partial_units > 0) {
        @memcpy(result[pos..][0..partial_slice.len], partial_slice);
        pos += partial_slice.len;
    }

    // Write original string
    @memcpy(result[pos..], s);
    return result;
}

/// Pad the end of a string to reach target_len using pad_str repeated.
/// Uses UTF-16 code unit indexing (JS semantics): target_len is in UTF-16 code units.
pub fn padEnd(alloc: Allocator, s: []const u8, target_len: i64, pad_str: []const u8) ![]const u8 {
    const s_utf16_len: usize = utf16Len(s);
    const pad_utf16_len: usize = utf16Len(pad_str);
    const target: usize = @intCast(@max(0, target_len));
    if (s_utf16_len >= target or pad_utf16_len == 0) return try alloc.dupe(u8, s);

    const pad_needed: usize = target - s_utf16_len; // in UTF-16 code units
    const full_reps = pad_needed / pad_utf16_len;
    const partial_units = pad_needed % pad_utf16_len;

    const partial_slice = if (partial_units > 0) firstUtf16CodeUnits(pad_str, partial_units) else &[0]u8{};

    const total_pad_bytes = full_reps * pad_str.len + partial_slice.len;
    const result = try alloc.alloc(u8, s.len + total_pad_bytes);

    // Write original string
    @memcpy(result[0..s.len], s);
    var pos: usize = s.len;

    // Write full repetitions of pad_str
    for (0..full_reps) |_| {
        @memcpy(result[pos..][0..pad_str.len], pad_str);
        pos += pad_str.len;
    }

    // Write partial prefix
    if (partial_units > 0) {
        @memcpy(result[pos..][0..partial_slice.len], partial_slice);
        pos += partial_slice.len;
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

// ── UTF-16 helper tests ──

test "utf16Len ASCII" {
    try std.testing.expectEqual(@as(usize, 5), utf16Len("hello"));
    try std.testing.expectEqual(@as(usize, 0), utf16Len(""));
}

test "utf16Len multi-byte" {
    // 'café' = 4 UTF-16 code units (c, a, f, é)
    try std.testing.expectEqual(@as(usize, 4), utf16Len("café"));
    // '😀' = 2 UTF-16 code units (surrogate pair)
    try std.testing.expectEqual(@as(usize, 2), utf16Len("😀"));
    // Mixed: 'Hi😀!' = 5 UTF-16 code units (H, i, high, low, !)
    try std.testing.expectEqual(@as(usize, 5), utf16Len("Hi😀!"));
}

test "utf16IndexToByteOffset ASCII" {
    try std.testing.expectEqual(@as(usize, 0), utf16IndexToByteOffset("abc", 0).?);
    try std.testing.expectEqual(@as(usize, 1), utf16IndexToByteOffset("abc", 1).?);
    try std.testing.expectEqual(@as(usize, 2), utf16IndexToByteOffset("abc", 2).?);
    try std.testing.expectEqual(@as(usize, 3), utf16IndexToByteOffset("abc", 3).?); // one-past-end
    try std.testing.expect(utf16IndexToByteOffset("abc", 4) == null); // out of bounds
}

test "utf16IndexToByteOffset multi-byte" {
    // 'café' in UTF-8: c(1) a(1) f(1) é(2) = 5 bytes
    try std.testing.expectEqual(@as(usize, 0), utf16IndexToByteOffset("café", 0).?); // 'c'
    try std.testing.expectEqual(@as(usize, 1), utf16IndexToByteOffset("café", 1).?); // 'a'
    try std.testing.expectEqual(@as(usize, 2), utf16IndexToByteOffset("café", 2).?); // 'f'
    try std.testing.expectEqual(@as(usize, 3), utf16IndexToByteOffset("café", 3).?); // 'é'
    try std.testing.expectEqual(@as(usize, 5), utf16IndexToByteOffset("café", 4).?); // one-past-end
}

test "utf16IndexToByteOffset surrogate pair" {
    // '😀' in UTF-8: 4 bytes, 2 UTF-16 code units
    // Both high and low surrogate map to byte offset 0 (same code point)
    try std.testing.expectEqual(@as(usize, 0), utf16IndexToByteOffset("😀", 0).?); // high surrogate
    try std.testing.expectEqual(@as(usize, 0), utf16IndexToByteOffset("😀", 1).?); // low surrogate → same byte offset
    try std.testing.expectEqual(@as(usize, 4), utf16IndexToByteOffset("😀", 2).?); // one-past-end
    try std.testing.expect(utf16IndexToByteOffset("😀", 3) == null); // out of bounds
}

test "byteOffsetToUtf16Index" {
    // ASCII: 1:1 mapping
    try std.testing.expectEqual(@as(usize, 0), byteOffsetToUtf16Index("abc", 0));
    try std.testing.expectEqual(@as(usize, 1), byteOffsetToUtf16Index("abc", 1));
    try std.testing.expectEqual(@as(usize, 3), byteOffsetToUtf16Index("abc", 3));
    // 'café': byte 0→0, byte 1→1, byte 2→2, byte 3→3, byte 5→4
    try std.testing.expectEqual(@as(usize, 3), byteOffsetToUtf16Index("café", 3));
    try std.testing.expectEqual(@as(usize, 4), byteOffsetToUtf16Index("café", 5));
}

// ── Fixed method tests with multi-byte chars ──

test "charAt UTF-8" {
    const alloc = std.testing.allocator;
    // 'café': charAt(3) should return "é"
    const result = try charAt(alloc, "café", 3);
    defer alloc.free(result);
    try std.testing.expectEqualStrings("é", result);
}

test "charAt out of bounds" {
    const result = try charAt(std.testing.allocator, "abc", 10);
    try std.testing.expectEqualStrings("", result);
}

test "at UTF-8 positive" {
    const alloc = std.testing.allocator;
    const result = try at(alloc, "café", 3);
    defer alloc.free(result);
    try std.testing.expectEqualStrings("é", result);
}

test "at UTF-8 negative" {
    const alloc = std.testing.allocator;
    // "café" has 4 UTF-16 code units, at(-1) should return "é"
    const result = try at(alloc, "café", -1);
    defer alloc.free(result);
    try std.testing.expectEqualStrings("é", result);
}

test "slice UTF-8" {
    // "café": slice(1,3) should return "af"
    try std.testing.expectEqualStrings("af", slice("café", 1, 3));
    // "café": slice(3) should return "é"
    try std.testing.expectEqualStrings("é", slice("café", 3, std.math.maxInt(i64)));
}

test "substring UTF-8" {
    // "café": substring(1,3) should return "af"
    try std.testing.expectEqualStrings("af", substring("café", 1, 3));
}

test "indexOf UTF-8" {
    // "café": indexOf("é") should return 3 (UTF-16 index)
    try std.testing.expectEqual(@as(i64, 3), indexOf("café", "é"));
}

test "lastIndexOf UTF-8" {
    // "caféé": lastIndexOf("é") should return 4 (second é is UTF-16 index 4)
    try std.testing.expectEqual(@as(i64, 4), lastIndexOf("caféé", "é"));
}

test "padStart UTF-8" {
    const alloc = std.testing.allocator;
    // "café" has 4 UTF-16 code units; padStart(6, " ") should add 2 spaces
    const result = try padStart(alloc, "café", 6, " ");
    defer alloc.free(result);
    try std.testing.expectEqualStrings("  café", result);
}

test "padEnd UTF-8" {
    const alloc = std.testing.allocator;
    // "café" has 4 UTF-16 code units; padEnd(6, " ") should add 2 spaces
    const result = try padEnd(alloc, "café", 6, " ");
    defer alloc.free(result);
    try std.testing.expectEqualStrings("café  ", result);
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
