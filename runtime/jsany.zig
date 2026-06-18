//! JsAny — general-purpose JS value type that covers primitives, dynamic arrays, and objects.
//! Arrays use std.ArrayList(JsAny), objects use std.StringHashMap(JsAny).
//! Both containers allow nesting (JsAny can contain JsAny).
//! Updated for Zig 0.16.0 ArrayList API (.empty, deinit(alloc), append(alloc, item)).

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsValue = @import("jsvalue.zig").JsValue;

/// Type alias for a dynamic JS array.
pub const JsArrayList = std.ArrayList(JsAny);

/// Type alias for a dynamic JS object (string-keyed map).
pub const JsObjectMap = std.StringHashMap(JsAny);

/// General-purpose JS value that can hold primitives, arrays, or objects.
pub const JsAny = union(enum) {
    value: JsValue,
    array: *JsArrayList,
    object: *JsObjectMap,
    null: void,

    // === Constructors ===

    pub fn fromI64(v: i64) JsAny {
        return .{ .value = .{ .int = v } };
    }

    pub fn fromF64(v: f64) JsAny {
        return .{ .value = .{ .float = v } };
    }

    pub fn fromBool(v: bool) JsAny {
        return .{ .value = .{ .bool = v } };
    }

    pub fn fromString(v: []const u8) JsAny {
        return .{ .value = .{ .string = v } };
    }

    pub fn fromValue(v: JsValue) JsAny {
        return .{ .value = v };
    }

    pub fn fromNull() JsAny {
        return .{ .null = {} };
    }

    /// Create a new empty array on the heap.
    pub fn newArray(alloc: Allocator) !JsAny {
        const arr = try alloc.create(JsArrayList);
        arr.* = .empty;
        return .{ .array = arr };
    }

    /// Create a new empty object on the heap.
    pub fn newObject(alloc: Allocator) !JsAny {
        const obj = try alloc.create(JsObjectMap);
        obj.* = JsObjectMap.init(alloc);
        return .{ .object = obj };
    }

    // === Type checks ===

    pub fn isValue(self: JsAny) bool {
        return self == .value;
    }

    pub fn isArray(self: JsAny) bool {
        return self == .array;
    }

    pub fn isObject(self: JsAny) bool {
        return self == .object;
    }

    pub fn isNull(self: JsAny) bool {
        return self == .null;
    }

    pub fn isString(self: JsAny) bool {
        return switch (self) {
            .value => |v| v == .string,
            else => false,
        };
    }

    pub fn isNumber(self: JsAny) bool {
        return switch (self) {
            .value => |v| v == .int or v == .float,
            else => false,
        };
    }

    // === Coercion to primitive ===

    pub fn asI64(self: JsAny) i64 {
        return switch (self) {
            .value => |v| v.asI64(),
            .array => |a| @intCast(a.items.len),
            .object => |o| @intCast(o.count()),
            .null => 0,
        };
    }

    pub fn asF64(self: JsAny) f64 {
        return switch (self) {
            .value => |v| v.asF64(),
            .array => |a| @floatFromInt(a.items.len),
            .object => |o| @floatFromInt(o.count()),
            .null => 0.0,
        };
    }

    pub fn asBool(self: JsAny) bool {
        return switch (self) {
            .value => |v| v.asBool(),
            .array => |a| a.items.len > 0,
            .object => |o| o.count() > 0,
            .null => false,
        };
    }

    pub fn asString(self: JsAny, alloc: Allocator) []const u8 {
        return switch (self) {
            .value => |v| v.asString(alloc),
            .array => |a| blk: {
                var buf: std.ArrayList(u8) = .empty;
                defer buf.deinit(alloc);
                buf.append(alloc, '[') catch break :blk "";
                for (a.items, 0..) |item, i| {
                    if (i > 0) buf.append(alloc, ',') catch break :blk "";
                    if (item.isString()) {
                        buf.append(alloc, '"') catch break :blk "";
                        buf.appendSlice(alloc, item.asString(alloc)) catch break :blk "";
                        buf.append(alloc, '"') catch break :blk "";
                    } else {
                        buf.appendSlice(alloc, item.asString(alloc)) catch break :blk "";
                    }
                }
                buf.append(alloc, ']') catch break :blk "";
                break :blk buf.toOwnedSlice(alloc) catch "";
            },
            .object => |o| blk: {
                var buf: std.ArrayList(u8) = .empty;
                defer buf.deinit(alloc);
                buf.append(alloc, '{') catch break :blk "";
                var iter = o.iterator();
                var first = true;
                while (iter.next()) |entry| {
                    if (!first) buf.append(alloc, ',') catch break :blk "";
                    first = false;
                    buf.append(alloc, '"') catch break :blk "";
                    buf.appendSlice(alloc, entry.key_ptr.*) catch break :blk "";
                    buf.appendSlice(alloc, "\":") catch break :blk "";
                    if (entry.value_ptr.isString()) {
                        buf.append(alloc, '"') catch break :blk "";
                        buf.appendSlice(alloc, entry.value_ptr.asString(alloc)) catch break :blk "";
                        buf.append(alloc, '"') catch break :blk "";
                    } else {
                        buf.appendSlice(alloc, entry.value_ptr.asString(alloc)) catch break :blk "";
                    }
                }
                buf.append(alloc, '}') catch break :blk "";
                break :blk buf.toOwnedSlice(alloc) catch "";
            },
            .null => "null",
        };
    }

    // === Downgrade to JsValue ===

    pub fn toValue(self: JsAny) JsValue {
        return switch (self) {
            .value => |v| v,
            .array => |a| .{ .int = @intCast(a.items.len) },
            .object => |o| .{ .int = @intCast(o.count()) },
            .null => .null,
        };
    }

    // === Arithmetic (JS semantics) ===

    pub fn add(self: JsAny, other: JsAny, alloc: Allocator) JsAny {
        // String concat: if either side is a string, concatenate
        if (self.isString() or other.isString()) {
            const s = self.asString(alloc);
            const o = other.asString(alloc);
            const result = std.fmt.allocPrint(alloc, "{s}{s}", .{ s, o }) catch "";
            return .{ .value = .{ .string = result } };
        }
        // Numeric: int + int = int, otherwise float
        if (self.isNumber() and other.isNumber()) {
            if (self == .value and self.value == .int and other == .value and other.value == .int) {
                return .{ .value = .{ .int = self.value.int + other.value.int } };
            }
            return .{ .value = .{ .float = self.asF64() + other.asF64() } };
        }
        // Array + Array → concat (JS semantics: converts to string first)
        // Fallback: string concat
        const s = self.asString(alloc);
        const o = other.asString(alloc);
        const result = std.fmt.allocPrint(alloc, "{s}{s}", .{ s, o }) catch "";
        return .{ .value = .{ .string = result } };
    }

    pub fn sub(self: JsAny, other: JsAny) JsAny {
        if (self.isNumber() and other.isNumber()) {
            if (self == .value and self.value == .int and other == .value and other.value == .int) {
                return .{ .value = .{ .int = self.value.int - other.value.int } };
            }
        }
        return .{ .value = .{ .float = self.asF64() - other.asF64() } };
    }

    pub fn mul(self: JsAny, other: JsAny) JsAny {
        if (self.isNumber() and other.isNumber()) {
            if (self == .value and self.value == .int and other == .value and other.value == .int) {
                return .{ .value = .{ .int = self.value.int * other.value.int } };
            }
        }
        return .{ .value = .{ .float = self.asF64() * other.asF64() } };
    }

    pub fn div(self: JsAny, other: JsAny) JsAny {
        const denom = other.asF64();
        if (denom == 0.0) return .{ .value = .{ .float = std.math.inf(f64) } };
        return .{ .value = .{ .float = self.asF64() / denom } };
    }

    pub fn rem(self: JsAny, other: JsAny) JsAny {
        const d = other.asF64();
        if (d == 0.0) return .{ .value = .{ .float = std.math.nan(f64) } };
        return .{ .value = .{ .float = @mod(self.asF64(), d) } };
    }

    pub fn neg(self: JsAny) JsAny {
        return switch (self) {
            .value => |v| switch (v) {
                .int => |i| .{ .value = .{ .int = -i } },
                else => .{ .value = .{ .float = -self.asF64() } },
            },
            else => .{ .value = .{ .float = -self.asF64() } },
        };
    }

    pub fn not(self: JsAny) JsAny {
        return .{ .value = .{ .bool = !self.asBool() } };
    }

    // === Comparison ===

    pub fn eq(self: JsAny, other: JsAny) bool {
        return self.toValue().eq(other.toValue());
    }

    pub fn neq(self: JsAny, other: JsAny) bool {
        return !self.eq(other);
    }

    pub fn lt(self: JsAny, other: JsAny) bool {
        return self.asF64() < other.asF64();
    }

    pub fn le(self: JsAny, other: JsAny) bool {
        return self.asF64() <= other.asF64();
    }

    pub fn gt(self: JsAny, other: JsAny) bool {
        return self.asF64() > other.asF64();
    }

    pub fn ge(self: JsAny, other: JsAny) bool {
        return self.asF64() >= other.asF64();
    }

    // === Array operations ===

    /// Append an item to the array. Auto-upgrades non-arrays to arrays.
    pub fn arrayPush(self: *JsAny, alloc: Allocator, item: JsAny) !void {
        switch (self.*) {
            .array => |a| try a.append(alloc, item),
            else => {
                var new_arr = try alloc.create(JsArrayList);
                new_arr.* = .empty;
                try new_arr.append(alloc, self.*);
                try new_arr.append(alloc, item);
                self.* = .{ .array = new_arr };
            },
        }
    }

    /// Get array element by index, returns null if out of bounds.
    pub fn arrayGet(self: JsAny, index: usize) ?JsAny {
        return switch (self) {
            .array => |a| if (index < a.items.len) a.items[index] else null,
            else => null,
        };
    }

    /// Set array element by index. Does nothing if not an array.
    pub fn arraySet(self: *JsAny, index: usize, item: JsAny) void {
        switch (self.*) {
            .array => |a| {
                if (index < a.items.len) a.items[index] = item;
            },
            else => {},
        }
    }

    /// Remove and return last element. Returns null if empty or not array.
    pub fn arrayPop(self: *JsAny) ?JsAny {
        return switch (self.*) {
            .array => |a| a.pop(),
            else => null,
        };
    }

    /// Get array length.
    pub fn arrayLen(self: JsAny) usize {
        return switch (self) {
            .array => |a| a.items.len,
            else => 0,
        };
    }

    // === Object operations ===

    /// Get object property by key. Returns null if not found or not object.
    pub fn objectGet(self: JsAny, key: []const u8) ?JsAny {
        return switch (self) {
            .object => |o| o.get(key),
            else => null,
        };
    }

    /// Set object property by key. Does nothing if not object.
    pub fn objectPut(self: *JsAny, key: []const u8, val: JsAny) !void {
        switch (self.*) {
            .object => |o| try o.put(key, val),
            else => {},
        }
    }

    /// Check if object has a key.
    pub fn objectHas(self: JsAny, key: []const u8) bool {
        return switch (self) {
            .object => |o| o.contains(key),
            else => false,
        };
    }

    /// Get object key count.
    pub fn objectLen(self: JsAny) usize {
        return switch (self) {
            .object => |o| o.count(),
            else => 0,
        };
    }

    // === Cleanup ===

    /// Deinit heap-allocated array or object.
    pub fn deinit(self: *JsAny, alloc: Allocator) void {
        switch (self.*) {
            .array => |a| {
                for (a.items) |*item| item.deinit(alloc);
                a.deinit(alloc);
                alloc.destroy(a);
            },
            .object => |o| {
                var iter = o.iterator();
                while (iter.next()) |entry| {
                    var val = entry.value_ptr.*;
                    val.deinit(alloc);
                }
                o.deinit();
                alloc.destroy(o);
            },
            else => {},
        }
        self.* = .{ .null = {} };
    }
};

// ── Tests ──

test "JsAny primitive constructors" {
    const a = JsAny.fromI64(42);
    try std.testing.expect(a.isValue());
    try std.testing.expect(a.isNumber());
    try std.testing.expectEqual(@as(i64, 42), a.asI64());

    const b = JsAny.fromF64(3.14);
    try std.testing.expect(b.isNumber());

    const c = JsAny.fromBool(true);
    try std.testing.expect(c.asBool());

    const d = JsAny.fromString("hello");
    try std.testing.expect(d.isString());

    const e = JsAny.fromNull();
    try std.testing.expect(e.isNull());
}

test "JsAny numeric arithmetic" {
    const a = JsAny.fromI64(10);
    const b = JsAny.fromI64(3);

    const sum = a.add(b, std.testing.allocator);
    try std.testing.expectEqual(@as(i64, 13), sum.value.int);

    const diff = a.sub(b);
    try std.testing.expectEqual(@as(i64, 7), diff.value.int);

    const product = a.mul(b);
    try std.testing.expectEqual(@as(i64, 30), product.value.int);
}

test "JsAny float arithmetic" {
    const a = JsAny.fromF64(10.0);
    const b = JsAny.fromF64(4.0);

    const quotient = a.div(b);
    try std.testing.expectEqual(@as(f64, 2.5), quotient.value.float);
}

test "JsAny string concat" {
    const alloc = std.testing.allocator;
    const a = JsAny.fromString("Hello, ");
    const b = JsAny.fromString("World!");
    const result = a.add(b, alloc);
    defer alloc.free(result.value.string);
    try std.testing.expectEqualStrings("Hello, World!", result.value.string);
}

test "JsAny comparison" {
    const a = JsAny.fromI64(5);
    const b = JsAny.fromI64(10);
    try std.testing.expect(a.lt(b));
    try std.testing.expect(b.gt(a));
    try std.testing.expect(a.le(a));
    try std.testing.expect(a.ge(a));
}

test "JsAny array operations" {
    const alloc = std.testing.allocator;
    var arr = try JsAny.newArray(alloc);
    defer arr.deinit(alloc);

    try arr.arrayPush(alloc, JsAny.fromI64(1));
    try arr.arrayPush(alloc, JsAny.fromI64(2));
    try arr.arrayPush(alloc, JsAny.fromI64(3));

    try std.testing.expectEqual(@as(usize, 3), arr.arrayLen());

    const first = arr.arrayGet(0).?;
    try std.testing.expectEqual(@as(i64, 1), first.value.int);

    const popped = arr.arrayPop().?;
    try std.testing.expectEqual(@as(i64, 3), popped.value.int);
    try std.testing.expectEqual(@as(usize, 2), arr.arrayLen());
}

test "JsAny object operations" {
    const alloc = std.testing.allocator;
    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);

    try obj.objectPut("name", JsAny.fromString("Zig"));
    try obj.objectPut("version", JsAny.fromI64(0));

    try std.testing.expect(obj.objectHas("name"));
    try std.testing.expectEqual(@as(usize, 2), obj.objectLen());

    const name = obj.objectGet("name").?;
    try std.testing.expectEqualStrings("Zig", name.value.string);
}

test "JsAny nested array in object" {
    const alloc = std.testing.allocator;
    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);

    var inner = try JsAny.newArray(alloc);
    try inner.arrayPush(alloc, JsAny.fromI64(1));
    try inner.arrayPush(alloc, JsAny.fromI64(2));
    try obj.objectPut("data", inner);

    const got = obj.objectGet("data").?;
    try std.testing.expect(got.isArray());
    try std.testing.expectEqual(@as(usize, 2), got.arrayLen());
}
