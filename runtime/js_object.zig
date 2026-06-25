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
