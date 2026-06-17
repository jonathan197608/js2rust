//! JS RegExp method implementations for Zig.
//! Simplified: uses substring matching (not full regex).
//! All allocating functions take `alloc: std.mem.Allocator` as first parameter.

const std = @import("std");
const Allocator = std.mem.Allocator;

/// RegExp.test — check if pattern matches anywhere in the subject.
/// Simplified: uses substring matching.
pub fn test_(subject: []const u8, pattern: []const u8) bool {
    return std.mem.indexOf(u8, subject, pattern) != null;
}

/// RegExp.exec — find first match, returns matched substring or null.
pub fn exec(alloc: Allocator, subject: []const u8, pattern: []const u8) !?[]const u8 {
    if (std.mem.indexOf(u8, subject, pattern)) |pos| {
        const match_len = pattern.len;
        return try alloc.dupe(u8, subject[pos .. pos + match_len]);
    }
    return null;
}

// ── Tests ──

test "test_" {
    try std.testing.expect(test_("hello world", "world"));
    try std.testing.expect(test_("hello world", "hello"));
    try std.testing.expect(!test_("hello world", "xyz"));
}

test "exec" {
    const result = try exec(std.testing.allocator, "hello world", "world");
    defer if (result) |r| std.testing.allocator.free(r);
    try std.testing.expect(result != null);
    try std.testing.expectEqualStrings("world", result.?);
}

test "exec no match" {
    const result = try exec(std.testing.allocator, "hello", "xyz");
    defer if (result) |r| std.testing.allocator.free(r);
    try std.testing.expectEqual(@as(?[]const u8, null), result);
}
