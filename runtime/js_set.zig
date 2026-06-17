//! JS Set implementation for Zig.
//! Uses std.AutoHashMap(i64, void) internally (set of i64 values).

const std = @import("std");
const Allocator = std.mem.Allocator;

pub const JsSet = struct {
    inner: std.AutoHashMap(i64, void),

    pub fn init(alloc: Allocator) JsSet {
        return JsSet{
            .inner = std.AutoHashMap(i64, void).init(alloc),
        };
    }

    pub fn deinit(self: *JsSet) void {
        self.inner.deinit();
    }

    /// Add a value. Returns nothing (like JS Set.add).
    pub fn add(self: *JsSet, value: i64) !void {
        try self.inner.put(value, {});
    }

    /// Check if value exists.
    pub fn has(self: *const JsSet, value: i64) bool {
        return self.inner.contains(value);
    }

    /// Remove a value. Returns true if value was present.
    pub fn delete(self: *JsSet, value: i64) bool {
        return self.inner.remove(value);
    }

    /// Remove all values.
    pub fn clear(self: *JsSet) void {
        self.inner.clearAndFree();
    }

    /// Number of elements.
    pub fn size(self: *const JsSet) usize {
        return self.inner.count();
    }
};

// ── Tests ──

test "JsSet add/has" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit();

    try s.add(1);
    try s.add(2);
    try std.testing.expect(s.has(1));
    try std.testing.expect(s.has(2));
    try std.testing.expect(!s.has(3));
}

test "JsSet delete" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit();

    try s.add(10);
    try std.testing.expect(s.delete(10));
    try std.testing.expect(!s.has(10));
    try std.testing.expect(!s.delete(10));
}

test "JsSet clear" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit();

    try s.add(1);
    try s.add(2);
    s.clear();
    try std.testing.expectEqual(@as(usize, 0), s.size());
}

test "JsSet size" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit();

    try std.testing.expectEqual(@as(usize, 0), s.size());
    try s.add(42);
    try std.testing.expectEqual(@as(usize, 1), s.size());
}
