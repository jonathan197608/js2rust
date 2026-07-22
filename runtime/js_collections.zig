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
        // R12-P1-3: Normalize JsAny.null and JsAny.value(.null) to the same
        // representation so they hash identically as Map/Set keys.
        const k: JsAny = if (key == .null) .{ .value = .null } else key;
        var hasher = std.hash.Wyhash.init(0);
        const tag_id = @intFromEnum(std.meta.activeTag(k));
        hasher.update(std.mem.asBytes(&tag_id));
        switch (k) {
            .value => |v| switch (v) {
                .int => |i| hasher.update(std.mem.asBytes(&i)),
                .float => |f| {
                    // SameValueZero: NaN === NaN → true. Use a canonical
                    // bit pattern for all NaN values so they hash identically.
                    if (std.math.isNan(f)) {
                        const canonical: f64 = std.math.nan(f64);
                        hasher.update(std.mem.asBytes(&canonical));
                    } else {
                        hasher.update(std.mem.asBytes(&f));
                    }
                },
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
        // R12-P1-3: Normalize JsAny.null and JsAny.value(.null) to the same
        // representation so they compare equal as Map/Set keys.
        const na: JsAny = if (a == .null) .{ .value = .null } else a;
        const nb: JsAny = if (b == .null) .{ .value = .null } else b;
        const tag_a = std.meta.activeTag(na);
        const tag_b = std.meta.activeTag(nb);
        if (tag_a != tag_b) return false;
        switch (na) {
            .value => |va| switch (nb.value) {
                .int => |ib| return switch (va) {
                    .int => |ia| return ia == ib,
                    else => false,
                },
                .float => |fb| return switch (va) {
                    .float => |fa| {
                        // SameValueZero: NaN === NaN → true
                        if (std.math.isNan(fa) and std.math.isNan(fb)) return true;
                        return fa == fb;
                    },
                    else => false,
                },
                .bool => |bb| return switch (va) {
                    .bool => |ba| return ba == bb,
                    else => false,
                },
                .string => |sb| return switch (va) {
                    .string => |sa| return std.mem.eql(u8, sa, sb),
                    else => false,
                },
                .null => return va == .null,
                .undefined => return va == .undefined,
            },
            .array => |pa| return pa == nb.array,
            .object => |po| return po == nb.object,
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

        /// Return ArrayList of keys. Caller must call deinit(alloc).
        pub fn keys(self: *const @This(), alloc: Allocator) !std.ArrayList(JsAny) {
            var result: std.ArrayList(JsAny) = .empty;
            try result.ensureTotalCapacity(alloc, self.inner.count());
            var iter = self.inner.keyIterator();
            while (iter.next()) |key_ptr| {
                try result.append(alloc, key_ptr.*);
            }
            return result;
        }

        /// Return ArrayList of values. Caller must call deinit(alloc).
        /// For Set (Value == void): identical to keys().
        /// For Map (Value == JsAny): returns the values.
        pub fn values(self: *const @This(), alloc: Allocator) !std.ArrayList(JsAny) {
            if (is_set) {
                return self.keys(alloc);
            } else {
                var result: std.ArrayList(JsAny) = .empty;
                try result.ensureTotalCapacity(alloc, self.inner.count());
                var iter = self.inner.valueIterator();
                while (iter.next()) |val_ptr| {
                    try result.append(alloc, val_ptr.*);
                }
                return result;
            }
        }

        /// Return ArrayList of [k, v] pairs (each pair is an ArrayList(JsAny)).
        /// Caller must deinit inner ArrayLists and the outer ArrayList.
        /// Set:  each pair is [value, value]  (MDN spec)
        /// Map:  each pair is [key, value]  (MDN spec)
        pub fn entries(self: *const @This(), alloc: Allocator) !std.ArrayList(std.ArrayList(JsAny)) {
            var result: std.ArrayList(std.ArrayList(JsAny)) = .empty;
            try result.ensureTotalCapacity(alloc, self.inner.count());
            if (is_set) {
                // Set: [value, value]
                var iter = self.inner.keyIterator();
                while (iter.next()) |key_ptr| {
                    var pair: std.ArrayList(JsAny) = .empty;
                    try pair.append(alloc, key_ptr.*);
                    try pair.append(alloc, key_ptr.*); // [value, value]
                    try result.append(alloc, pair);
                }
            } else {
                // Map: [key, value]
                var iter = self.inner.iterator();
                while (iter.next()) |entry| {
                    var pair: std.ArrayList(JsAny) = .empty;
                    try pair.append(alloc, entry.key_ptr.*);
                    try pair.append(alloc, entry.value_ptr.*);
                    try result.append(alloc, pair);
                }
            }
            return result;
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

    var vals = try s.values(alloc);
    defer vals.deinit(alloc);

    try std.testing.expectEqual(@as(usize, 3), vals.items.len);
}

test "JsSet keys() same as values()" {
    const alloc = std.testing.allocator;
    var s = JsSet.init(alloc);
    defer s.deinit(std.testing.allocator);

    try s.add(JsAny.fromI64(1));
    try s.add(JsAny.fromI64(2));

    var keys = try s.keys(alloc);
    defer keys.deinit(alloc);
    var vals = try s.values(alloc);
    defer vals.deinit(alloc);

    try std.testing.expectEqual(keys.items.len, vals.items.len);
}

test "JsSet entries()" {
    const alloc = std.testing.allocator;
    var s = JsSet.init(alloc);
    defer s.deinit(std.testing.allocator);

    try s.add(JsAny.fromI64(5));
    try s.add(JsAny.fromI64(10));

    var ents = try s.entries(alloc);
    defer {
        for (ents.items) |*pair| pair.deinit(alloc);
        ents.deinit(alloc);
    }

    try std.testing.expectEqual(@as(usize, 2), ents.items.len);
    for (ents.items) |pair| {
        try std.testing.expectEqual(@as(usize, 2), pair.items.len);
        // In JS Set entries, pair[0] == pair[1]
        try std.testing.expect(pair.items[0].eq(pair.items[1]));
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

    var keys = try m.keys(alloc);
    defer keys.deinit(alloc);

    try std.testing.expectEqual(@as(usize, 2), keys.items.len);
}

test "JsMap values()" {
    const alloc = std.testing.allocator;
    var m = JsMap.init(alloc);
    defer m.deinit(std.testing.allocator);

    try m.set(JsAny.fromI64(1), JsAny.fromString("one"));
    try m.set(JsAny.fromI64(2), JsAny.fromString("two"));

    var vals = try m.values(alloc);
    defer vals.deinit(alloc);

    try std.testing.expectEqual(@as(usize, 2), vals.items.len);
}

test "JsMap entries()" {
    const alloc = std.testing.allocator;
    var m = JsMap.init(alloc);
    defer m.deinit(std.testing.allocator);

    try m.set(JsAny.fromI64(5), JsAny.fromString("five"));
    try m.set(JsAny.fromI64(10), JsAny.fromString("ten"));

    var ents = try m.entries(alloc);
    defer {
        for (ents.items) |*pair| pair.deinit(alloc);
        ents.deinit(alloc);
    }

    try std.testing.expectEqual(@as(usize, 2), ents.items.len);
    for (ents.items) |pair| {
        try std.testing.expectEqual(@as(usize, 2), pair.items.len);
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

test "NaN SameValueZero: NaN === NaN in Map/Set (R7-6)" {
    // SameValueZero semantics: NaN is equal to itself for Map/Set keys.
    // Pre-fix: eql used IEEE == which makes NaN != NaN, so two NaN keys
    // were stored as separate entries instead of one.
    const alloc = std.testing.allocator;

    // Set with NaN key
    var s = JsSet.init(alloc);
    defer s.deinit(alloc);
    try s.add(JsAny.fromF64(std.math.nan(f64)));
    try s.add(JsAny.fromF64(std.math.nan(f64))); // should be a no-op (duplicate)
    try std.testing.expectEqual(@as(usize, 1), s.size());
    try std.testing.expect(s.has(JsAny.fromF64(std.math.nan(f64))));

    // Map with NaN key
    var m = JsMap.init(alloc);
    defer m.deinit(alloc);
    try m.set(JsAny.fromF64(std.math.nan(f64)), JsAny.fromI64(1));
    try m.set(JsAny.fromF64(std.math.nan(f64)), JsAny.fromI64(2)); // overwrites
    try std.testing.expectEqual(@as(usize, 1), m.size());
    const v = m.get(JsAny.fromF64(std.math.nan(f64)));
    try std.testing.expect(v.eq(JsAny.fromI64(2)));
}

test "null key normalization: JsAny.null and JsAny.value(.null) are same key (R12-P1-3)" {
    const alloc = std.testing.allocator;

    // Map: set with JsAny.fromNull(), then has with JsAny.fromValue(.null)
    var m = JsMap.init(alloc);
    defer m.deinit(alloc);
    try m.set(JsAny.fromNull(), JsAny.fromI64(1));
    try std.testing.expect(m.has(JsAny.fromValue(.null)));
    try std.testing.expectEqual(@as(usize, 1), m.size());

    // Overwrite via the other representation
    try m.set(JsAny.fromValue(.null), JsAny.fromI64(2));
    try std.testing.expectEqual(@as(usize, 1), m.size());
    const v = m.get(JsAny.fromNull());
    try std.testing.expect(v.eq(JsAny.fromI64(2)));

    // Set: add both representations → size 1
    var s = JsSet.init(alloc);
    defer s.deinit(alloc);
    try s.add(JsAny.fromNull());
    try s.add(JsAny.fromValue(.null));
    try std.testing.expectEqual(@as(usize, 1), s.size());

    // null ≠ undefined as keys
    try s.add(JsAny.fromUndefined());
    try std.testing.expectEqual(@as(usize, 2), s.size());
}
