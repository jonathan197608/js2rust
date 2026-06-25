//! JS Set implementation for Zig.
//! Uses std.HashMap(JsAny, void, JsAnyHashMapContext, ...) internally.
//! Supports arbitrary JsAny values (primitives, strings, arrays, objects).
//! String values are compared by content; arrays/objects by reference (pointer equality).

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsAny = @import("jsany.zig").JsAny;

/// Custom hash map context for JsAny.
/// - hash: primitives hash by value; strings by content; pointers by address
/// - eql:  SameValueZero semantics (primitives by value, strings by content, pointers by address)
const JsAnyHashMapContext = struct {
    pub fn hash(_: JsAnyHashMapContext, key: JsAny) u64 {
        // Mix the tag first so different variants don't collide.
        var hasher = std.hash.Wyhash.init(0);
        const tag_id = @intFromEnum(std.meta.activeTag(key));
        hasher.update(std.mem.asBytes(&tag_id));
        switch (key) {
            .value => |v| switch (v) {
                .int => |i| hasher.update(std.mem.asBytes(&i)),
                .float => |f| hasher.update(std.mem.asBytes(&f)),
                .bool => |b| {
                    const byte: u8 = if (b) 1 else 0;
                    hasher.update(&[_]u8{byte});
                },
                .string => |s| hasher.update(s),
                .null => {},
                .undefined => {},
            },
            .array => |a| {
                const addr = @intFromPtr(a);
                hasher.update(std.mem.asBytes(&addr));
            },
            .object => |o| {
                const addr = @intFromPtr(o);
                hasher.update(std.mem.asBytes(&addr));
            },
            .null => {},
        }
        return hasher.final();
    }

    pub fn eql(_: JsAnyHashMapContext, a: JsAny, b: JsAny) bool {
        const tag_a = std.meta.activeTag(a);
        const tag_b = std.meta.activeTag(b);
        if (tag_a != tag_b) return false;
        switch (a) {
            .value => |va| switch (va) {
                .int => |ia| return b.value.int == ia,
                .float => |fa| return b.value.float == fa,
                .bool => |ba| return b.value.bool == ba,
                .string => |sa| return std.mem.eql(u8, sa, b.value.string),
                .null => return true,
                .undefined => return true,
            },
            .array => |pa| return pa == b.array,
            .object => |po| return po == b.object,
            .null => return true,
        }
    }
};

pub const JsSet = struct {
    inner: std.HashMap(JsAny, void, JsAnyHashMapContext, std.hash_map.default_max_load_percentage),

    pub fn init(alloc: Allocator) JsSet {
        return JsSet{
            .inner = std.HashMap(JsAny, void, JsAnyHashMapContext, std.hash_map.default_max_load_percentage).init(alloc),
        };
    }

    pub fn deinit(self: *JsSet) void {
        self.inner.deinit();
    }

    /// Add a value. Returns nothing (like JS Set.add).
    pub fn add(self: *JsSet, value: JsAny) !void {
        try self.inner.put(value, {});
    }

    /// Check if value exists.
    pub fn has(self: *const JsSet, value: JsAny) bool {
        return self.inner.contains(value);
    }

    /// Remove a value. Returns true if value was present.
    pub fn delete(self: *JsSet, value: JsAny) bool {
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

    /// Return array of values (JS Set.values() / Set.keys()).
    /// Caller must free the returned slice.
    pub fn values(self: *const JsSet, alloc: Allocator) ![]JsAny {
        var result = try alloc.alloc(JsAny, self.inner.count());
        var i: usize = 0;
        var iter = self.inner.keyIterator();
        while (iter.next()) |key_ptr| {
            result[i] = key_ptr.*;
            i += 1;
        }
        return result;
    }

    /// Return array of keys. For JS Set this is the same as values().
    /// Caller must free the returned slice.
    pub fn keys(self: *const JsSet, alloc: Allocator) ![]JsAny {
        return self.values(alloc);
    }

    /// Return array of [value, value] pairs (JS Set.entries()).
    /// Caller must free each inner slice and the outer slice.
    pub fn entries(self: *const JsSet, alloc: Allocator) ![][]JsAny {
        var result = try alloc.alloc([]JsAny, self.inner.count());
        var i: usize = 0;
        var iter = self.inner.keyIterator();
        while (iter.next()) |key_ptr| {
            const pair = try alloc.alloc(JsAny, 2);
            pair[0] = key_ptr.*;
            pair[1] = key_ptr.*;
            result[i] = pair;
            i += 1;
        }
        return result;
    }
};

// ── Tests ──

test "JsSet add/has (i64)" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit();

    try s.add(JsAny.fromI64(1));
    try s.add(JsAny.fromI64(2));
    try std.testing.expect(s.has(JsAny.fromI64(1)));
    try std.testing.expect(s.has(JsAny.fromI64(2)));
    try std.testing.expect(!s.has(JsAny.fromI64(3)));
}

test "JsSet add/has (string)" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit();

    try s.add(JsAny.fromString("hello"));
    try s.add(JsAny.fromString("world"));
    try std.testing.expect(s.has(JsAny.fromString("hello")));
    try std.testing.expect(s.has(JsAny.fromString("world")));
    try std.testing.expect(!s.has(JsAny.fromString("missing")));
}

test "JsSet add/has (mixed types)" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit();

    try s.add(JsAny.fromI64(42));
    try s.add(JsAny.fromString("answer"));
    try s.add(JsAny.fromBool(true));
    try s.add(JsAny.fromNull());

    try std.testing.expect(s.has(JsAny.fromI64(42)));
    try std.testing.expect(s.has(JsAny.fromString("answer")));
    try std.testing.expect(s.has(JsAny.fromBool(true)));
    try std.testing.expect(s.has(JsAny.fromNull()));
}

test "JsSet duplicate values ignored" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit();

    try s.add(JsAny.fromI64(1));
    try s.add(JsAny.fromI64(1)); // duplicate
    try std.testing.expectEqual(@as(usize, 1), s.size());
}

test "JsSet delete" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit();

    try s.add(JsAny.fromI64(10));
    try std.testing.expect(s.delete(JsAny.fromI64(10)));
    try std.testing.expect(!s.has(JsAny.fromI64(10)));
    try std.testing.expect(!s.delete(JsAny.fromI64(10)));
}

test "JsSet clear" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit();

    try s.add(JsAny.fromI64(1));
    try s.add(JsAny.fromI64(2));
    s.clear();
    try std.testing.expectEqual(@as(usize, 0), s.size());
}

test "JsSet size" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit();

    try std.testing.expectEqual(@as(usize, 0), s.size());
    try s.add(JsAny.fromI64(42));
    try std.testing.expectEqual(@as(usize, 1), s.size());
}

test "JsSet values()" {
    const alloc = std.testing.allocator;
    var s = JsSet.init(alloc);
    defer s.deinit();

    try s.add(JsAny.fromI64(10));
    try s.add(JsAny.fromI64(20));
    try s.add(JsAny.fromString("hello"));

    const vals = try s.values(alloc);
    defer alloc.free(vals);

    try std.testing.expectEqual(@as(usize, 3), vals.len);
}

test "JsSet keys() same as values()" {
    const alloc = std.testing.allocator;
    var s = JsSet.init(alloc);
    defer s.deinit();

    try s.add(JsAny.fromI64(1));
    try s.add(JsAny.fromI64(2));

    const keys = try s.keys(alloc);
    defer alloc.free(keys);
    const vals = try s.values(alloc);
    defer alloc.free(vals);

    try std.testing.expectEqual(keys.len, vals.len);
}

test "JsSet entries()" {
    const alloc = std.testing.allocator;
    var s = JsSet.init(alloc);
    defer s.deinit();

    try s.add(JsAny.fromI64(5));
    try s.add(JsAny.fromI64(10));

    const ents = try s.entries(alloc);
    defer {
        for (ents) |pair| alloc.free(pair);
        alloc.free(ents);
    }

    try std.testing.expectEqual(@as(usize, 2), ents.len);
    for (ents) |pair| {
        try std.testing.expectEqual(@as(usize, 2), pair.len);
        // In JS Set entries, pair[0] == pair[1]
        try std.testing.expect(pair[0].eq(pair[1]));
    }
}
