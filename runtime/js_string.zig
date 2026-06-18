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
