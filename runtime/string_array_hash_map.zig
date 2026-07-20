//! Managed insertion-order-preserving string-keyed map wrapper.
//!
//! Zig 0.16.0's stdlib only exports `std.StringArrayHashMapUnmanaged` — an
//! "unmanaged" type where every allocating method takes an `Allocator`
//! argument. There is no managed `std.StringArrayHashMap`.
//!
//! This wrapper stores the allocator so callers can use a managed-style API
//! (`init(alloc)`, `deinit()`, `put(k, v)`) while still getting
//! insertion-order-preserving iteration — which JavaScript Object semantics
//! require (`Object.keys`, `for...in`, `JSON.stringify` must emit keys in
//! insertion order).
//!
//! The underlying unmanaged type preserves insertion order, and `remove()`
//! delegates to `orderedRemove()` (O(N), preserves order) rather than
//! `swapRemove()` (O(1), reorders) to match JavaScript `delete` semantics.

const std = @import("std");
const Allocator = std.mem.Allocator;

/// A managed insertion-order-preserving map from `[]const u8` to `V`.
///
/// Wraps `std.StringArrayHashMapUnmanaged(V)` and stores an `Allocator` so
/// that `init`/`deinit`/`put` mirror the managed `std.StringHashMap` API.
pub fn StringArrayHashMap(comptime V: type) type {
    return struct {
        const Self = @This();
        pub const Unmanaged = std.StringArrayHashMapUnmanaged(V);
        /// Re-export so callers can reference `StringArrayHashMap(V).Entry`.
        pub const Entry = Unmanaged.Entry;
        /// Re-export so callers can reference `StringArrayHashMap(V).Iterator`.
        pub const Iterator = Unmanaged.Iterator;

        unmanaged: Unmanaged = .empty,
        allocator: Allocator,

        /// Create an empty map backed by `allocator`.
        pub fn init(allocator: Allocator) Self {
            return .{
                .unmanaged = .empty,
                .allocator = allocator,
            };
        }

        /// Free the map's backing storage.
        /// Does NOT free key strings or heap-allocated values — callers are
        /// responsible for releasing those before calling `deinit` (mirrors
        /// `std.StringHashMap.deinit` semantics).
        pub fn deinit(self: *Self) void {
            self.unmanaged.deinit(self.allocator);
        }

        /// Look up `key`, returning an optional copy of the stored value.
        pub fn get(self: Self, key: []const u8) ?V {
            return self.unmanaged.get(key);
        }

        /// Look up `key`, returning an optional pointer to the stored value.
        /// The pointer is invalidated by any insertion or removal.
        pub fn getPtr(self: Self, key: []const u8) ?*V {
            return self.unmanaged.getPtr(key);
        }

        /// Insert or overwrite the value for `key`.
        pub fn put(self: *Self, key: []const u8, value: V) Allocator.Error!void {
            try self.unmanaged.put(self.allocator, key, value);
        }

        /// Return `true` if `key` is present.
        pub fn contains(self: Self, key: []const u8) bool {
            return self.unmanaged.contains(key);
        }

        /// Return the number of stored entries.
        pub fn count(self: Self) usize {
            return self.unmanaged.count();
        }

        /// Remove `key`, preserving insertion order of remaining entries.
        /// Returns `true` if the key was found and removed, `false` otherwise.
        /// Uses `orderedRemove` (O(N)) rather than `swapRemove` (O(1)) so that
        /// iteration order is unchanged after a JavaScript `delete`.
        pub fn remove(self: *Self, key: []const u8) bool {
            return self.unmanaged.orderedRemove(key);
        }

        /// Return an iterator over entries in insertion order.
        pub fn iterator(self: Self) Iterator {
            return self.unmanaged.iterator();
        }
    };
}

// ── Tests ──────────────────────────────────────────────────────────────

test "init and deinit empty" {
    const alloc = std.testing.allocator;
    var map = StringArrayHashMap(i32).init(alloc);
    defer map.deinit();
    try std.testing.expectEqual(@as(usize, 0), map.count());
}

test "put and get preserves insertion order" {
    const alloc = std.testing.allocator;
    var map = StringArrayHashMap(i32).init(alloc);
    defer map.deinit();
    try map.put("c", 3);
    try map.put("a", 1);
    try map.put("b", 2);

    // Keys should be iterated in insertion order: c, a, b — NOT sorted.
    var iter = map.iterator();
    var collected = std.ArrayList([]const u8).empty;
    defer collected.deinit(alloc);
    while (iter.next()) |entry| {
        try collected.append(alloc, entry.key_ptr.*);
    }
    try std.testing.expectEqual(@as(usize, 3), collected.items.len);
    try std.testing.expectEqualStrings("c", collected.items[0]);
    try std.testing.expectEqualStrings("a", collected.items[1]);
    try std.testing.expectEqualStrings("b", collected.items[2]);
}

test "get and getPtr" {
    const alloc = std.testing.allocator;
    var map = StringArrayHashMap(i32).init(alloc);
    defer map.deinit();
    try map.put("x", 42);

    try std.testing.expectEqual(@as(?i32, 42), map.get("x"));
    try std.testing.expectEqual(@as(?i32, null), map.get("missing"));

    const ptr = map.getPtr("x").?;
    try std.testing.expectEqual(@as(i32, 42), ptr.*);
    ptr.* = 99;
    try std.testing.expectEqual(@as(?i32, 99), map.get("x"));
}

test "contains and count" {
    const alloc = std.testing.allocator;
    var map = StringArrayHashMap(i32).init(alloc);
    defer map.deinit();
    try map.put("a", 1);
    try map.put("b", 2);

    try std.testing.expect(map.contains("a"));
    try std.testing.expect(!map.contains("z"));
    try std.testing.expectEqual(@as(usize, 2), map.count());
}

test "remove preserves insertion order" {
    const alloc = std.testing.allocator;
    var map = StringArrayHashMap(i32).init(alloc);
    defer map.deinit();
    try map.put("a", 1);
    try map.put("b", 2);
    try map.put("c", 3);
    try map.put("d", 4);

    // Remove middle entry "b" — remaining order should be a, c, d.
    try std.testing.expect(map.remove("b"));
    try std.testing.expect(!map.remove("b")); // already removed
    try std.testing.expectEqual(@as(usize, 3), map.count());

    var iter = map.iterator();
    var collected = std.ArrayList([]const u8).empty;
    defer collected.deinit(alloc);
    while (iter.next()) |entry| {
        try collected.append(alloc, entry.key_ptr.*);
    }
    try std.testing.expectEqual(@as(usize, 3), collected.items.len);
    try std.testing.expectEqualStrings("a", collected.items[0]);
    try std.testing.expectEqualStrings("c", collected.items[1]);
    try std.testing.expectEqualStrings("d", collected.items[2]);
}

test "overwrite keeps original position" {
    const alloc = std.testing.allocator;
    var map = StringArrayHashMap(i32).init(alloc);
    defer map.deinit();
    try map.put("a", 1);
    try map.put("b", 2);
    try map.put("c", 3);

    // Overwrite "b" — position should be unchanged (a, b, c).
    try map.put("b", 99);
    try std.testing.expectEqual(@as(?i32, 99), map.get("b"));

    var iter = map.iterator();
    var collected = std.ArrayList([]const u8).empty;
    defer collected.deinit(alloc);
    while (iter.next()) |entry| {
        try collected.append(alloc, entry.key_ptr.*);
    }
    try std.testing.expectEqualStrings("a", collected.items[0]);
    try std.testing.expectEqualStrings("b", collected.items[1]);
    try std.testing.expectEqualStrings("c", collected.items[2]);
}
