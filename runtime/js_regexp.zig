//! JS RegExp method implementations for Zig.
//! Simplified: uses substring matching (not full regex) for local functions,
//! but JsRegExp struct delegates to host C ABI functions for full regex support.
//! All allocating functions take `alloc: std.mem.Allocator` as first parameter.

const std = @import("std");
const Allocator = std.mem.Allocator;

/// RegExp.test — check if pattern matches anywhere in the subject.
/// Simplified: uses substring matching.
pub fn test_(subject: []const u8, pattern: []const u8) bool {
    return std.mem.indexOf(u8, subject, pattern) != null;
}

/// RegExp object wrapping a pattern string.
/// Delegates to host C ABI functions for full regex matching (fancy-regex).
pub const JsRegExp = struct {
    pattern: []const u8,

    /// Create a new RegExp from a pattern string.
    /// The pattern is owned by this struct and must be freed with deinit().
    pub fn init(alloc: Allocator, pattern: []const u8) !JsRegExp {
        const owned = try alloc.dupe(u8, pattern);
        return JsRegExp{ .pattern = owned };
    }

    /// Release the owned pattern string.
    pub fn deinit(self: *JsRegExp, alloc: Allocator) void {
        alloc.free(self.pattern);
        self.pattern = &.{};
    }

    /// Test if the pattern matches anywhere in the subject.
    /// Returns true if a match is found.
    // NOTE: renamed from `test` because `test` is a Zig keyword (used for test blocks).
    pub fn isMatch(self: *const JsRegExp, subject: []const u8) bool {
        return host_regex_test(self.pattern.ptr, self.pattern.len, subject.ptr, subject.len);
    }

    /// Match the pattern against the subject, returning captured groups.
    /// Returns null if no match, or an array of matched substrings
    /// (index 0 = full match, indices 1+ = capture groups).
    pub fn match_(self: *const JsRegExp, alloc: Allocator, subject: []const u8) !?[][]const u8 {
        var count: usize = 0;
        const result = host_regex_match(self.pattern.ptr, self.pattern.len, subject.ptr, subject.len, &count);
        if (count == 0) return null;

        const bytes = result.ptr[0..@intCast(result.len)];
        var matches = std.ArrayList([]const u8).init(alloc);
        errdefer matches.deinit();

        var start: usize = 0;
        for (0..count) |_| {
            var end: usize = start;
            while (end < bytes.len and bytes[end] != 0) : (end += 1) {}
            try matches.append(bytes[start..end]);
            start = end + 1;
        }

        return try matches.toOwnedSlice();
    }

    /// Execute the pattern against the subject, returning captured groups.
    /// Returns null if no match, or an array of matched substrings
    /// (index 0 = full match, indices 1+ = capture groups).
    pub fn exec(self: *const JsRegExp, alloc: Allocator, subject: []const u8) !?[][]const u8 {
        var count: usize = 0;
        const result = host_regex_match(self.pattern.ptr, self.pattern.len, subject.ptr, subject.len, &count);
        if (count == 0) return null;

        const bytes = result.ptr[0..@intCast(result.len)];
        var matches = std.ArrayList([]const u8).init(alloc);
        errdefer matches.deinit();

        var start: usize = 0;
        for (0..count) |_| {
            var end: usize = start;
            while (end < bytes.len and bytes[end] != 0) : (end += 1) {}
            try matches.append(bytes[start..end]);
            start = end + 1;
        }

        return try matches.toOwnedSlice();
    }

    /// Search for the pattern in the subject.
    /// Returns the index of the first match, or -1 if no match.
    pub fn search(self: *const JsRegExp, subject: []const u8) i64 {
        return host_regex_search(self.pattern.ptr, self.pattern.len, subject.ptr, subject.len);
    }
};

/// Standalone exec for RegExp literal codegen: /pattern/.exec(str)
/// Returns null if no match, or an array of matched substrings.
pub fn execLiteral(alloc: Allocator, subject: []const u8, pattern: []const u8) !?[][]const u8 {
    var count: usize = 0;
    const result = host_regex_match(pattern.ptr, pattern.len, subject.ptr, subject.len, &count);
    if (count == 0) return null;

    const bytes = result.ptr[0..@intCast(result.len)];
    var matches = std.ArrayList([]const u8).init(alloc);
    errdefer matches.deinit();

    var start: usize = 0;
    for (0..count) |_| {
        var end: usize = start;
        while (end < bytes.len and bytes[end] != 0) : (end += 1) {}
        try matches.append(bytes[start..end]);
        start = end + 1;
    }

    return try matches.toOwnedSlice();
}

// ── Host C ABI function declarations ──

extern fn host_regex_test(
    pattern_ptr: [*]const u8,
    pattern_len: usize,
    text_ptr: [*]const u8,
    text_len: usize,
) bool;

extern fn host_regex_search(
    pattern_ptr: [*]const u8,
    pattern_len: usize,
    text_ptr: [*]const u8,
    text_len: usize,
) i64;

extern fn host_regex_match(
    pattern_ptr: [*]const u8,
    pattern_len: usize,
    text_ptr: [*]const u8,
    text_len: usize,
    out_count: *usize,
) extern struct { ptr: [*]const u8, len: isize };

// ── Tests ──

test "test_" {
    try std.testing.expect(test_("hello world", "world"));
    try std.testing.expect(test_("hello world", "hello"));
    try std.testing.expect(!test_("hello world", "xyz"));
}

test "execLiteral" {
    const result = try execLiteral(std.testing.allocator, "hello world", "world");
    defer {
        if (result) |r| {
            for (r) |s| std.testing.allocator.free(s);
            std.testing.allocator.free(r);
        }
    }
    try std.testing.expect(result != null);
    try std.testing.expectEqualStrings("world", result.?[0]);
}

test "execLiteral no match" {
    const result = try execLiteral(std.testing.allocator, "hello", "xyz");
    defer {
        if (result) |r| {
            for (r) |s| std.testing.allocator.free(s);
            std.testing.allocator.free(r);
        }
    }
    try std.testing.expectEqual(@as(?[][]const u8, null), result);
}
