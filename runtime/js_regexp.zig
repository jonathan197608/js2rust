//! JS RegExp method implementations for Zig.
//! Simplified: uses substring matching (not full regex) for local functions,
//! but JsRegExp struct delegates to host C ABI functions for full regex support.
//! All allocating functions take `alloc: std.mem.Allocator` as first parameter.

const std = @import("std");
const Allocator = std.mem.Allocator;
const js_allocator = @import("js_allocator.zig");

/// RegExp.test — check if pattern matches anywhere in the subject.
/// Simplified: uses substring matching.
pub fn test_(subject: []const u8, pattern: []const u8) bool {
    return std.mem.indexOf(u8, subject, pattern) != null;
}

/// RegExp object wrapping a pattern string.
/// Delegates to host C ABI functions for full regex matching (fancy-regex).
pub const JsRegExp = struct {
    pattern: []const u8,
    flags: []const u8,
    global: bool,

    /// Create a new RegExp from a pattern string and flags string.
    pub fn init(alloc: Allocator, pattern: []const u8, flags: []const u8) !JsRegExp {
        const owned_pattern = try alloc.dupe(u8, pattern);
        const owned_flags = try alloc.dupe(u8, flags);
        return JsRegExp{
            .pattern = owned_pattern,
            .flags = owned_flags,
            .global = std.mem.indexOfScalar(u8, flags, 'g') != null,
        };
    }

    /// Backward-compatible init without flags.
    pub fn initNoFlags(alloc: Allocator, pattern: []const u8) !JsRegExp {
        return init(alloc, pattern, "");
    }

    /// Release the owned pattern and flags strings.
    /// Under the arena allocator, free() is a no-op — isNoOpFree() skips the work.
    pub fn deinit(self: *JsRegExp, alloc: Allocator) void {
        if (js_allocator.isNoOpFree(alloc)) return;
        alloc.free(self.pattern);
        alloc.free(self.flags);
        self.pattern = &.{};
        self.flags = &.{};
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
        var matches = std.ArrayList([]const u8).empty;
        errdefer matches.deinit(alloc);

        var start: usize = 0;
        for (0..count) |_| {
            var end: usize = start;
            while (end < bytes.len and bytes[end] != 0) : (end += 1) {}
            try matches.append(alloc, bytes[start..end]);
            start = end + 1;
        }

        return try matches.toOwnedSlice(alloc);
    }

    /// Execute the pattern against the subject, returning captured groups.
    /// Returns null if no match, or an array of matched substrings
    /// (index 0 = full match, indices 1+ = capture groups).
    pub fn exec(self: *const JsRegExp, alloc: Allocator, subject: []const u8) !?[][]const u8 {
        var count: usize = 0;
        const result = host_regex_match(self.pattern.ptr, self.pattern.len, subject.ptr, subject.len, &count);
        if (count == 0) return null;

        const bytes = result.ptr[0..@intCast(result.len)];
        var matches = std.ArrayList([]const u8).empty;
        errdefer matches.deinit(alloc);

        var start: usize = 0;
        for (0..count) |_| {
            var end: usize = start;
            while (end < bytes.len and bytes[end] != 0) : (end += 1) {}
            try matches.append(alloc, bytes[start..end]);
            start = end + 1;
        }

        return try matches.toOwnedSlice(alloc);
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
    var matches: std.ArrayList([]const u8) = std.ArrayList([]const u8).empty;
    errdefer matches.deinit(alloc);

    var start: usize = 0;
    for (0..count) |_| {
        var end: usize = start;
        while (end < bytes.len and bytes[end] != 0) : (end += 1) {}
        try matches.append(alloc, bytes[start..end]);
        start = end + 1;
    }

    return try matches.toOwnedSlice(alloc);
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
// When running under zig test with host_regex_stubs, the host functions
// always return false / null.  Detect this at test startup and skip
// regex-dependent tests gracefully via error.SkipZigTest.

test "test_" {
    // Quick stub-detect: host_regex_test always returns false under stubs.
    if (!host_regex_test("world", 5, "hello world", 11)) return error.SkipZigTest;
    try std.testing.expect(test_("hello world", "hello"));
    try std.testing.expect(!test_("hello world", "xyz"));
}

test "execLiteral" {
    if (!host_regex_test("world", 5, "hello world", 11)) return error.SkipZigTest;
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
    if (!host_regex_test("world", 5, "hello world", 11)) return error.SkipZigTest;
    const result = try execLiteral(std.testing.allocator, "hello", "xyz");
    defer {
        if (result) |r| {
            for (r) |s| std.testing.allocator.free(s);
            std.testing.allocator.free(r);
        }
    }
    try std.testing.expectEqual(@as(?[][]const u8, null), result);
}
