//! JS Object static method implementations for Zig.
//! Works with std.StringHashMap(JsValue) for dynamic objects.
//! All allocating functions take `alloc: std.mem.Allocator` as first parameter.

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsValue = @import("jsvalue.zig").JsValue;

const JsValueHashMap = std.StringHashMap(JsValue);

/// Object.keys — return array of string keys from a HashMap.
pub fn keys(alloc: Allocator, obj: *const JsValueHashMap) ![][]const u8 {
    var kiter = obj.iterator();
    var list = std.ArrayList([]const u8).empty;
    errdefer list.deinit(alloc);
    while (kiter.next()) |entry| {
        const key_copy = try alloc.dupe(u8, entry.key_ptr.*);
        try list.append(alloc, key_copy);
    }
    return list.toOwnedSlice(alloc);
}

/// Object.values — return array of JsValue values from a HashMap.
pub fn values(alloc: Allocator, obj: *const JsValueHashMap) ![]JsValue {
    var kiter = obj.iterator();
    var list = std.ArrayList(JsValue).empty;
    errdefer list.deinit(alloc);
    while (kiter.next()) |entry| {
        try list.append(alloc, entry.value_ptr.*);
    }
    return list.toOwnedSlice(alloc);
}

/// Object.entries — return array of [key, value] pairs from a HashMap.
/// Each pair is a struct { key: []const u8, value: JsValue }.
pub const Entry = struct { key: []const u8, value: JsValue };

pub fn entries(alloc: Allocator, obj: *const JsValueHashMap) ![]Entry {
    var kiter = obj.iterator();
    var list = std.ArrayList(Entry).empty;
    errdefer list.deinit(alloc);
    while (kiter.next()) |entry| {
        const key_copy = try alloc.dupe(u8, entry.key_ptr.*);
        errdefer alloc.free(key_copy);
        try list.append(alloc, .{ .key = key_copy, .value = entry.value_ptr.* });
    }
    return list.toOwnedSlice(alloc);
}

/// Object.assign — copy entries from source to target HashMap.
pub fn assign(target: *JsValueHashMap, source: *const JsValueHashMap) !void {
    var siter = source.iterator();
    while (siter.next()) |entry| {
        try target.put(entry.key_ptr.*, entry.value_ptr.*);
    }
}

/// Object.hasOwn — check if key exists in HashMap.
pub fn hasOwn(obj: *const JsValueHashMap, key: []const u8) bool {
    return obj.contains(key);
}

// ── Tests ──

test "keys" {
    const alloc = std.testing.allocator;
    var obj = JsValueHashMap.init(alloc);
    defer obj.deinit();
    try obj.put("name", JsValue{ .string = "zig" });
    try obj.put("version", JsValue{ .int = 1 });

    const k = try keys(alloc, &obj);
    defer {
        for (k) |key| alloc.free(key);
        alloc.free(k);
    }
    try std.testing.expect(k.len >= 2);
}

test "values" {
    const alloc = std.testing.allocator;
    var obj = JsValueHashMap.init(alloc);
    defer obj.deinit();
    try obj.put("x", JsValue{ .int = 10 });
    try obj.put("y", JsValue{ .int = 20 });

    const v = try values(alloc, &obj);
    defer alloc.free(v);
    try std.testing.expect(v.len >= 2);
}

test "entries" {
    const alloc = std.testing.allocator;
    var obj = JsValueHashMap.init(alloc);
    defer obj.deinit();
    try obj.put("name", JsValue{ .string = "zig" });
    try obj.put("version", JsValue{ .int = 1 });

    const e = try entries(alloc, &obj);
    defer {
        for (e) |entry| alloc.free(entry.key);
        alloc.free(e);
    }
    try std.testing.expect(e.len >= 2);
}

test "assign" {
    const alloc = std.testing.allocator;
    var target = JsValueHashMap.init(alloc);
    defer target.deinit();
    var source = JsValueHashMap.init(alloc);
    defer source.deinit();
    try source.put("a", JsValue{ .int = 1 });

    try assign(&target, &source);
    try std.testing.expectEqual(@as(i64, 1), target.get("a").?.int);
}

/// Object.create(proto) — create a new object with optional prototype.
/// In our simplified implementation without prototype chain:
/// - If proto is null: return empty HashMap
/// - If proto is an object: create new HashMap and copy properties from proto
pub fn create(alloc: Allocator, proto: ?*const JsValueHashMap) !JsValueHashMap {
    var obj = JsValueHashMap.init(alloc);
    if (proto) |p| {
        // Copy all properties from prototype
        var iter = p.iterator();
        while (iter.next()) |entry| {
            try obj.put(entry.key_ptr.*, entry.value_ptr.*);
        }
    }
    return obj;
}

/// Object.seal(obj) — prevent extensions (simplified: no-op in Zig).
/// In JS, sealed objects cannot add new properties.
/// Our generated Zig code is immutable by default, so this is a no-op.
pub fn seal(obj: *JsValueHashMap) void {
    _ = obj;
    // No-op: Zig HashMap can still be modified, but JS code can't after seal()
}

/// Object.defineProperty(obj, key, descriptor) — define a property.
/// Simplified: just set the value (ignore descriptor).
pub fn defineProperty(obj: *JsValueHashMap, key: []const u8, value: JsValue) !void {
    try obj.put(key, value);
}

/// Object.getPrototypeOf(obj) — get prototype (simplified: always null).
/// In our implementation, objects don't have prototype chains.
pub fn getPrototypeOf(obj: *const JsValueHashMap) ?*const JsValueHashMap {
    _ = obj;
    return null;
}

// ── Tests for new Object methods (Phase 5) ──

test "create with null proto" {
    const alloc = std.testing.allocator;
    var obj = try create(alloc, null);
    defer obj.deinit();
    try std.testing.expect(obj.count() == 0);
}

test "create with proto" {
    const alloc = std.testing.allocator;
    var proto = JsValueHashMap.init(alloc);
    defer proto.deinit();
    try proto.put("name", JsValue{ .string = "proto" });
    try proto.put("version", JsValue{ .int = 1 });

    var obj = try create(alloc, &proto);
    defer obj.deinit();
    try std.testing.expect(obj.contains("name"));
    try std.testing.expect(obj.contains("version"));
}

test "seal is no-op" {
    const alloc = std.testing.allocator;
    var obj = JsValueHashMap.init(alloc);
    defer obj.deinit();
    try obj.put("x", JsValue{ .int = 1 });

    seal(&obj); // Should not panic or error
    try std.testing.expectEqual(@as(i64, 1), obj.get("x").?.int);
}

test "defineProperty" {
    const alloc = std.testing.allocator;
    var obj = JsValueHashMap.init(alloc);
    defer obj.deinit();

    try defineProperty(&obj, "name", JsValue{ .string = "zig" });
    try std.testing.expectEqualStrings("zig", obj.get("name").?.string);
}

test "getPrototypeOf returns null" {
    const alloc = std.testing.allocator;
    var obj = JsValueHashMap.init(alloc);
    defer obj.deinit();

    const proto = getPrototypeOf(&obj);
    try std.testing.expect(proto == null);
}
