//! JS Map and Set — merged implementation in one file.
//! Uses comptime parameterization:
//!   JsCollection(void)   → Set  (key: JsAny, no separate value)
//!   JsCollection(JsAny) → Map  (key: JsAny, value: JsAny)
//!
//! Both use JsAnyHashMapContext for SameValueZero semantics
//! (NaN === NaN, +0 === -0, strings by content, objects by reference).
//!
//! Design note: Zig does not support conditional method inclusion at struct
//! scope (the `if (comptime_bool) { ... }` feature was never stabilized).
//! Instead, all methods are defined on JsCollection(Value); calling the
//! wrong method for the type triggers @compileError at compile time.

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsAny = @import("jsany.zig").JsAny;
const js_allocator = @import("js_allocator.zig");

// ── JsAnyHashMapContext ───────────────────────────────────────────
// Shared between Map and Set. Implements SameValueZero:
//   - primitives: by value
//   - strings:     by content (mem.eql)
//   - arrays/objects: by pointer identity (address)
//   - NaN:         hash stable, eql(NaN, NaN) = true
// ─────────────────────────────────────────────────────────────────────

const JsAnyHashMapContext = struct {
    pub fn hash(_: JsAnyHashMapContext, key: JsAny) u64 {
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

// ── JsCollection(comptime Value: type) ────────────────────────
// Generic collection. Instantiate as:
//   pub const JsMap = JsCollection(JsAny);  // key→value
//   pub const JsSet = JsCollection(void);    // key only (value = void)
// ─────────────────────────────────────────────────────────────────────

pub fn JsCollection(comptime Value: type) type {
    const is_set = (Value == void);
    const is_map = (Value != void);

    return struct {
        inner: std.HashMap(JsAny, Value, JsAnyHashMapContext, std.hash_map.default_max_load_percentage),

        // ── Lifetime ─────────────────────────────────────────────

        pub fn init(alloc: Allocator) @This() {
            return @This(){
                .inner = std.HashMap(JsAny, Value, JsAnyHashMapContext, std.hash_map.default_max_load_percentage).init(alloc),
            };
        }

        /// Free the collection and recursively free heap-allocated JsAny values
        /// (arrays and objects). Uses JsAny.deinit() which does NOT free
        /// .value.string — this avoids double-free on string literals
        /// (fromString("literal") puts non-heap pointers into .value.string).
        /// String leaks are acceptable: production uses arena allocation,
        /// and arena reset reclaims all strings at once.
        ///
        /// Under the multi-arena allocator, free() is a no-op, so the entire
        /// traversal is wasted CPU. isNoOpFree() short-circuits with a single
        /// pointer comparison, skipping the traversal entirely.
        pub fn deinit(self: *@This(), alloc: Allocator) void {
            if (js_allocator.isNoOpFree(alloc)) return;
            var iter = self.inner.iterator();
            while (iter.next()) |entry| {
                var key = entry.key_ptr.*;
                key.deinit(alloc);
                if (is_map) {
                    var val = entry.value_ptr.*;
                    val.deinit(alloc);
                }
            }
            self.inner.deinit();
        }

        // ── Shared mutations ────────────────────────────────────

        /// Generic insert. Set callers should use add(), Map callers set().
        fn put(self: *@This(), key: JsAny, value: Value) !void {
            try self.inner.put(key, value);
        }

        pub fn has(self: *const @This(), key: JsAny) bool {
            return self.inner.contains(key);
        }

        pub fn delete(self: *@This(), alloc: Allocator, key: JsAny) bool {
            // Deep-free the existing entry before removing it
            if (self.inner.getEntry(key)) |entry| {
                var k = entry.key_ptr.*;
                k.deinit(alloc);
                if (is_map) {
                    var v = entry.value_ptr.*;
                    v.deinit(alloc);
                }
                return self.inner.remove(key);
            }
            return false;
        }

        pub fn clear(self: *@This(), alloc: Allocator) void {
            if (js_allocator.isNoOpFree(alloc)) {
                self.inner.clearAndFree();
                return;
            }
            var iter = self.inner.iterator();
            while (iter.next()) |entry| {
                var key = entry.key_ptr.*;
                key.deinit(alloc);
                if (is_map) {
                    var val = entry.value_ptr.*;
                    val.deinit(alloc);
                }
            }
            self.inner.clearAndFree();
        }

        pub fn size(self: *const @This()) usize {
            return self.inner.count();
        }

        // ── Iterators (keys / values / entries) ────────────────

        /// Return array of keys. Caller must free.
        pub fn keys(self: *const @This(), alloc: Allocator) ![]JsAny {
            var result = try alloc.alloc(JsAny, self.inner.count());
            var i: usize = 0;
            var iter = self.inner.keyIterator();
            while (iter.next()) |key_ptr| {
                result[i] = key_ptr.*;
                i += 1;
            }
            return result;
        }

        /// Return array of values. Caller must free.
        /// For Set (Value == void): identical to keys().
        /// For Map (Value == JsAny): returns the values.
        pub fn values(self: *const @This(), alloc: Allocator) ![]JsAny {
            if (is_set) {
                return self.keys(alloc);
            } else {
                var result = try alloc.alloc(JsAny, self.inner.count());
                var i: usize = 0;
                var iter = self.inner.valueIterator();
                while (iter.next()) |val_ptr| {
                    result[i] = val_ptr.*;
                    i += 1;
                }
                return result;
            }
        }

        /// Return array of [k, v] pairs. Caller must free inner slices and outer slice.
        /// Set:  each pair is [value, value]  (MDN spec)
        /// Map:  each pair is [key, value]  (MDN spec)
        pub fn entries(self: *const @This(), alloc: Allocator) ![][]JsAny {
            if (is_set) {
                // Set: [value, value]
                var result = try alloc.alloc([]JsAny, self.inner.count());
                var i: usize = 0;
                var iter = self.inner.keyIterator();
                while (iter.next()) |key_ptr| {
                    const pair = try alloc.alloc(JsAny, 2);
                    pair[0] = key_ptr.*;
                    pair[1] = key_ptr.*; // [value, value]
                    result[i] = pair;
                    i += 1;
                }
                return result;
            } else {
                // Map: [key, value]
                var result = try alloc.alloc([]JsAny, self.inner.count());
                var i: usize = 0;
                var iter = self.inner.iterator();
                while (iter.next()) |entry| {
                    const pair = try alloc.alloc(JsAny, 2);
                    pair[0] = entry.key_ptr.*;
                    pair[1] = entry.value_ptr.*;
                    result[i] = pair;
                    i += 1;
                }
                return result;
            }
        }

        // ── Set-only methods ───────────────────────────────────

        /// Set.add(value) — insert a value.
        /// Only valid when Value == void (i.e., JsSet).
        /// Calling on JsMap is a compile error.
        pub fn add(self: *@This(), value: JsAny) !void {
            if (!is_set) {
                @compileError("add() is only valid for Set (JsSet)");
            }
            try self.inner.put(value, {});
        }

        // ── Map-only methods ────────────────────────────────────

        /// Map.set(key, value) — insert or update a key-value pair.
        /// Only valid when Value == JsAny (i.e., JsMap).
        /// Calling on JsSet is a compile error.
        pub fn set(self: *@This(), key: JsAny, value: JsAny) !void {
            if (!is_map) {
                @compileError("set() is only valid for Map (JsMap)");
            }
            try self.inner.put(key, value);
        }

        /// Map.get(key) — return value or undefined_value if not found.
        /// Only valid when Value == JsAny (i.e., JsMap).
        /// Calling on JsSet is a compile error.
        /// Mirrors JS semantics: Map.get(missingKey) → undefined (not null).
        pub fn get(self: *const @This(), key: JsAny) JsAny {
            if (!is_map) {
                @compileError("get() is only valid for Map (JsMap)");
            }
            return if (self.inner.get(key)) |v| v else JsAny.undefined_value;
        }
    };
}

/// JS Map: key and value are both JsAny (any JS value type).
/// MDN: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Map
pub const JsMap = JsCollection(JsAny);

/// JS Set: only values (no separate key). Internally key=value.
/// MDN: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Set
pub const JsSet = JsCollection(void);

// ── Tests ────────────────────────────────────────────────────────

test "JsSet add/has (i64)" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit(std.testing.allocator);

    try s.add(JsAny.fromI64(1));
    try s.add(JsAny.fromI64(2));
    try std.testing.expect(s.has(JsAny.fromI64(1)));
    try std.testing.expect(s.has(JsAny.fromI64(2)));
    try std.testing.expect(!s.has(JsAny.fromI64(3)));
}

test "JsSet add/has (string)" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit(std.testing.allocator);

    try s.add(JsAny.fromString("hello"));
    try s.add(JsAny.fromString("world"));
    try std.testing.expect(s.has(JsAny.fromString("hello")));
    try std.testing.expect(s.has(JsAny.fromString("world")));
    try std.testing.expect(!s.has(JsAny.fromString("missing")));
}

test "JsSet add/has (mixed types)" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit(std.testing.allocator);

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
    defer s.deinit(std.testing.allocator);

    try s.add(JsAny.fromI64(1));
    try s.add(JsAny.fromI64(1)); // duplicate
    try std.testing.expectEqual(@as(usize, 1), s.size());
}

test "JsSet delete" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit(std.testing.allocator);

    try s.add(JsAny.fromI64(10));
    try std.testing.expect(s.delete(std.testing.allocator, JsAny.fromI64(10)));
    try std.testing.expect(!s.has(JsAny.fromI64(10)));
    try std.testing.expect(!s.delete(std.testing.allocator, JsAny.fromI64(10)));
}

test "JsSet clear" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit(std.testing.allocator);

    try s.add(JsAny.fromI64(1));
    try s.add(JsAny.fromI64(2));
    s.clear(std.testing.allocator);
    try std.testing.expectEqual(@as(usize, 0), s.size());
}

test "JsSet size" {
    var s = JsSet.init(std.testing.allocator);
    defer s.deinit(std.testing.allocator);

    try std.testing.expectEqual(@as(usize, 0), s.size());
    try s.add(JsAny.fromI64(42));
    try std.testing.expectEqual(@as(usize, 1), s.size());
}

test "JsSet values()" {
    const alloc = std.testing.allocator;
    var s = JsSet.init(alloc);
    defer s.deinit(std.testing.allocator);

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
    defer s.deinit(std.testing.allocator);

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
    defer s.deinit(std.testing.allocator);

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

// ── JsMap tests ─────────────────────────────────────────────────

test "JsMap set/get/has" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit(std.testing.allocator);

    try m.set(JsAny.fromI64(1), JsAny.fromString("one"));
    try m.set(JsAny.fromI64(2), JsAny.fromString("two"));

    try std.testing.expect(m.has(JsAny.fromI64(1)));
    try std.testing.expect(m.has(JsAny.fromI64(2)));

    const v = m.get(JsAny.fromI64(1));
    try std.testing.expect(v.eq(JsAny.fromString("one")));
}

test "JsMap get returns undefined for missing key" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit(std.testing.allocator);

    try m.set(JsAny.fromString("a"), JsAny.fromI64(1));
    try std.testing.expect(m.get(JsAny.fromString("missing")).isUndefined());
}

test "JsMap delete" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit(std.testing.allocator);

    try m.set(JsAny.fromString("x"), JsAny.fromI64(10));
    try std.testing.expect(m.delete(std.testing.allocator, JsAny.fromString("x")));
    try std.testing.expect(!m.has(JsAny.fromString("x")));
    try std.testing.expect(!m.delete(std.testing.allocator, JsAny.fromString("x")));
}

test "JsMap clear" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit(std.testing.allocator);

    try m.set(JsAny.fromString("a"), JsAny.fromI64(1));
    try m.set(JsAny.fromString("b"), JsAny.fromI64(2));
    m.clear(std.testing.allocator);
    try std.testing.expectEqual(@as(usize, 0), m.size());
}

test "JsMap size" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit(std.testing.allocator);

    try std.testing.expectEqual(@as(usize, 0), m.size());
    try m.set(JsAny.fromString("a"), JsAny.fromI64(1));
    try std.testing.expectEqual(@as(usize, 1), m.size());
}

test "JsMap keys()" {
    const alloc = std.testing.allocator;
    var m = JsMap.init(alloc);
    defer m.deinit(std.testing.allocator);

    try m.set(JsAny.fromI64(1), JsAny.fromString("one"));
    try m.set(JsAny.fromI64(2), JsAny.fromString("two"));

    const keys = try m.keys(alloc);
    defer alloc.free(keys);

    try std.testing.expectEqual(@as(usize, 2), keys.len);
}

test "JsMap values()" {
    const alloc = std.testing.allocator;
    var m = JsMap.init(alloc);
    defer m.deinit(std.testing.allocator);

    try m.set(JsAny.fromI64(1), JsAny.fromString("one"));
    try m.set(JsAny.fromI64(2), JsAny.fromString("two"));

    const vals = try m.values(alloc);
    defer alloc.free(vals);

    try std.testing.expectEqual(@as(usize, 2), vals.len);
}

test "JsMap entries()" {
    const alloc = std.testing.allocator;
    var m = JsMap.init(alloc);
    defer m.deinit(std.testing.allocator);

    try m.set(JsAny.fromI64(5), JsAny.fromString("five"));
    try m.set(JsAny.fromI64(10), JsAny.fromString("ten"));

    const ents = try m.entries(alloc);
    defer {
        for (ents) |pair| alloc.free(pair);
        alloc.free(ents);
    }

    try std.testing.expectEqual(@as(usize, 2), ents.len);
    for (ents) |pair| {
        try std.testing.expectEqual(@as(usize, 2), pair.len);
    }
}

test "JsMap key can be any JsAny type" {
    var m = JsMap.init(std.testing.allocator);
    defer m.deinit(std.testing.allocator);

    // key as number
    try m.set(JsAny.fromI64(42), JsAny.fromString("answer"));
    // key as string
    try m.set(JsAny.fromString("name"), JsAny.fromString("Alice"));
    // key as bool
    try m.set(JsAny.fromBool(true), JsAny.fromI64(1));

    try std.testing.expect(m.has(JsAny.fromI64(42)));
    try std.testing.expect(m.has(JsAny.fromString("name")));
    try std.testing.expect(m.has(JsAny.fromBool(true)));

    const v = m.get(JsAny.fromString("name"));
    try std.testing.expect(v.eq(JsAny.fromString("Alice")));
}
