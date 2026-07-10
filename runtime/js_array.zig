//! JS Array method implementations for Zig.
//! Works with []const i64 slices.
//! All allocating functions take `alloc: std.mem.Allocator` as first parameter.

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsAny = @import("jsany.zig").JsAny;

/// Array.isArray — check if value is a JsAny array.
pub fn isArray(value: JsAny) bool {
    return value.isArray();
}

/// Array.from(arrayLike) — create ArrayList(JsAny) from array-like value.
/// Handles: JsAny array (copy), string (each char as string), object with length.
pub fn from(alloc: Allocator, arrayLike: JsAny) !std.ArrayList(JsAny) {
    var result = std.ArrayList(JsAny).empty;
    errdefer result.deinit(alloc);

    // If already an array, copy elements
    if (arrayLike.isArray()) {
        const src = arrayLike.array.*;
        try result.ensureTotalCapacity(alloc, src.items.len);
        for (src.items) |item| {
            result.appendAssumeCapacity(item);
        }
        return result;
    }

    // If string, split into characters
    if (arrayLike.isString()) {
        const str = arrayLike.value.string;
        try result.ensureTotalCapacity(alloc, str.len);
        for (str) |ch| {
            const chr = try alloc.dupe(u8, &[1]u8{ch});
            result.appendAssumeCapacity(JsAny.fromString(chr));
        }
        return result;
    }

    // If object with length property, treat as array-like
    if (arrayLike.isObject()) {
        const obj = arrayLike.object.*;
        if (obj.get("length")) |len_val| {
            const len = @as(usize, @intCast(len_val.asI64()));
            try result.ensureTotalCapacity(alloc, len);
            var i: usize = 0;
            while (i < len) : (i += 1) {
                const key = try std.fmt.allocPrint(alloc, "{d}", .{i});
                defer alloc.free(key);
                if (obj.get(key)) |item| {
                    result.appendAssumeCapacity(item);
                } else {
                    result.appendAssumeCapacity(JsAny.undefined_value);
                }
            }
            return result;
        }
    }

    // Fallback: empty array
    return result;
}

/// Array.of(items) — create ArrayList(JsAny) from slice of JsAny.
pub fn of(alloc: Allocator, items: []const JsAny) !std.ArrayList(JsAny) {
    var result = std.ArrayList(JsAny).empty;
    errdefer result.deinit(alloc);
    try result.ensureTotalCapacity(alloc, items.len);
    for (items) |item| {
        result.appendAssumeCapacity(item);
    }
    return result;
}

/// Array.push — append a value, returns new slice.
pub fn push(alloc: Allocator, arr: []const i64, val: i64) ![]const i64 {
    const result = try alloc.alloc(i64, arr.len + 1);
    @memcpy(result[0..arr.len], arr);
    result[arr.len] = val;
    return result;
}

/// Array.pop — return last element (or null if empty).
pub fn pop(arr: []const i64) ?i64 {
    if (arr.len == 0) return null;
    return arr[arr.len - 1];
}

/// Array.shift — return first element (or null if empty).
pub fn shift(arr: []const i64) ?i64 {
    if (arr.len == 0) return null;
    return arr[0];
}

/// Array.unshift — prepend value, returns new slice.
pub fn unshift(alloc: Allocator, arr: []const i64, val: i64) ![]const i64 {
    const result = try alloc.alloc(i64, arr.len + 1);
    result[0] = val;
    @memcpy(result[1..], arr);
    return result;
}

/// Array.reverse — return new reversed slice.
pub fn reverse(alloc: Allocator, arr: []const i64) ![]const i64 {
    const result = try alloc.alloc(i64, arr.len);
    for (arr, 0..) |_, i| {
        result[i] = arr[arr.len - 1 - i];
    }
    return result;
}

/// Array.sort — return new ascending-sorted slice.
pub fn sort(alloc: Allocator, arr: []const i64) ![]const i64 {
    const result = try alloc.dupe(i64, arr);
    std.mem.sort(i64, result, {}, comptime std.sort.asc(i64));
    return result;
}

/// Array.flat — for uniform-typed arrays (i64, f64, etc.) this is an identity
/// operation since elements cannot be nested sub-arrays. The depth parameter
/// is accepted but has no effect on flat scalar arrays.
pub fn flat(alloc: Allocator, arr: []const i64, _depth: i64) ![]const i64 {
    _ = _depth;
    return try alloc.dupe(i64, arr);
}

/// Array.flatMap runtime fallback for scalar types without callback inlining.
/// With callback inlining (ArrayCallbackKind::FlatMap), the emit layer generates
/// an inline for-loop instead of calling this function.
pub fn flatMap(alloc: Allocator, arr: []const i64) ![]const i64 {
    return try alloc.dupe(i64, arr);
}

/// Array.keys() — returns new ArrayList(JsAny) containing indices.
/// JavaScript: arr.keys() returns an iterator of indices.
/// Our implementation: returns an array of indices.
pub fn keys(alloc: Allocator, arr: *const std.ArrayList(JsAny)) !std.ArrayList(JsAny) {
    var result = std.ArrayList(JsAny).empty;
    errdefer result.deinit(alloc);
    try result.ensureTotalCapacity(alloc, arr.items.len);
    for (0..arr.items.len) |i| {
        result.appendAssumeCapacity(JsAny.fromI64(@intCast(i)));
    }
    return result;
}

/// Array.values() — returns new ArrayList(JsAny) containing values.
/// JavaScript: arr.values() returns an iterator of values.
/// Our implementation: returns an array of values.
pub fn values(alloc: Allocator, arr: *const std.ArrayList(JsAny)) !std.ArrayList(JsAny) {
    var result = std.ArrayList(JsAny).empty;
    errdefer result.deinit(alloc);
    try result.ensureTotalCapacity(alloc, arr.items.len);
    for (arr.items) |item| {
        result.appendAssumeCapacity(item);
    }
    return result;
}

/// Array.entries() — returns new ArrayList(JsAny) containing [index, value] pairs.
/// JavaScript: arr.entries() returns an iterator of [index, value] pairs.
/// Our implementation: returns an array of arrays, where each inner array is [index, value].
pub fn entries(alloc: Allocator, arr: *const std.ArrayList(JsAny)) !std.ArrayList(JsAny) {
    var result = std.ArrayList(JsAny).empty;
    errdefer result.deinit(alloc);
    try result.ensureTotalCapacity(alloc, arr.items.len);
    for (arr.items, 0..) |item, i| {
        var pair = std.ArrayList(JsAny).empty;
        errdefer pair.deinit(alloc);
        try pair.append(alloc, JsAny.fromI64(@intCast(i)));
        try pair.append(alloc, item);

        // Allocate pair on heap and wrap in JsAny.array
        const pair_ptr = try alloc.create(std.ArrayList(JsAny));
        pair_ptr.* = pair;
        result.appendAssumeCapacity(JsAny{ .array = pair_ptr });
    }
    return result;
}

// ── Tests ──

test "reverse" {
    const result = try reverse(std.testing.allocator, &[_]i64{ 1, 2, 3 });
    defer std.testing.allocator.free(result);
    try std.testing.expectEqual(@as(i64, 3), result[0]);
    try std.testing.expectEqual(@as(i64, 2), result[1]);
    try std.testing.expectEqual(@as(i64, 1), result[2]);
}

test "sort" {
    const result = try sort(std.testing.allocator, &[_]i64{ 3, 1, 4, 1, 5 });
    defer std.testing.allocator.free(result);
    try std.testing.expectEqual(@as(i64, 1), result[0]);
    try std.testing.expectEqual(@as(i64, 5), result[4]);
}

test "shift" {
    const arr = &[_]i64{ 10, 20, 30 };
    try std.testing.expectEqual(@as(i64, 10), shift(arr).?);
    try std.testing.expectEqual(@as(?i64, null), shift(&[_]i64{}));
}

test "unshift" {
    const result = try unshift(std.testing.allocator, &[_]i64{ 2, 3 }, 1);
    defer std.testing.allocator.free(result);
    try std.testing.expectEqual(@as(usize, 3), result.len);
    try std.testing.expectEqual(@as(i64, 1), result[0]);
}

test "flat" {
    const result = try flat(std.testing.allocator, &[_]i64{ 1, 2, 3 }, 1);
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualSlices(i64, &[_]i64{ 1, 2, 3 }, result);
}

test "flatMap" {
    const result = try flatMap(std.testing.allocator, &[_]i64{ 1, 2, 3 });
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualSlices(i64, &[_]i64{ 1, 2, 3 }, result);
}

test "keys" {
    const alloc = std.testing.allocator;
    var arr = std.ArrayList(JsAny).empty;
    defer arr.deinit(alloc);
    try arr.append(alloc, JsAny.fromI64(10));
    try arr.append(alloc, JsAny.fromI64(20));
    try arr.append(alloc, JsAny.fromI64(30));

    var result = try keys(alloc, &arr);
    defer result.deinit(alloc);
    defer for (result.items) |*item| item.deinit(alloc);

    try std.testing.expectEqual(@as(usize, 3), result.items.len);
    try std.testing.expectEqual(@as(i64, 0), result.items[0].asI64());
    try std.testing.expectEqual(@as(i64, 1), result.items[1].asI64());
    try std.testing.expectEqual(@as(i64, 2), result.items[2].asI64());
}

test "values" {
    const alloc = std.testing.allocator;
    var arr = std.ArrayList(JsAny).empty;
    defer arr.deinit(alloc);
    try arr.append(alloc, JsAny.fromI64(10));
    try arr.append(alloc, JsAny.fromI64(20));
    try arr.append(alloc, JsAny.fromI64(30));

    var result = try values(alloc, &arr);
    defer result.deinit(alloc);
    defer for (result.items) |*item| item.deinit(alloc);

    try std.testing.expectEqual(@as(usize, 3), result.items.len);
    try std.testing.expectEqual(@as(i64, 10), result.items[0].asI64());
    try std.testing.expectEqual(@as(i64, 20), result.items[1].asI64());
    try std.testing.expectEqual(@as(i64, 30), result.items[2].asI64());
}

test "entries" {
    const alloc = std.testing.allocator;
    var arr = std.ArrayList(JsAny).empty;
    defer arr.deinit(alloc);
    try arr.append(alloc, JsAny.fromI64(10));
    try arr.append(alloc, JsAny.fromI64(20));
    try arr.append(alloc, JsAny.fromI64(30));

    var result = try entries(alloc, &arr);
    defer result.deinit(alloc);
    defer for (result.items) |*item| item.deinit(alloc);

    try std.testing.expectEqual(@as(usize, 3), result.items.len);

    // Check first entry: [0, 10]
    try std.testing.expect(result.items[0].isArray());
    const entry0 = result.items[0].array.*;
    try std.testing.expectEqual(@as(usize, 2), entry0.items.len);
    try std.testing.expectEqual(@as(i64, 0), entry0.items[0].asI64());
    try std.testing.expectEqual(@as(i64, 10), entry0.items[1].asI64());

    // Check second entry: [1, 20]
    try std.testing.expect(result.items[1].isArray());
    const entry1 = result.items[1].array.*;
    try std.testing.expectEqual(@as(usize, 2), entry1.items.len);
    try std.testing.expectEqual(@as(i64, 1), entry1.items[0].asI64());
    try std.testing.expectEqual(@as(i64, 20), entry1.items[1].asI64());
}

test "isArray" {
    const alloc = std.testing.allocator;

    // Array should return true
    var arr = try JsAny.newArray(alloc);
    defer arr.deinit(alloc);
    try std.testing.expect(isArray(arr));

    // String should return false
    const str = JsAny.fromString("hello");
    try std.testing.expect(!isArray(str));

    // Number should return false
    const num = JsAny.fromI64(42);
    try std.testing.expect(!isArray(num));

    // Object should return false
    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);
    try std.testing.expect(!isArray(obj));

    // Null should return false
    const null_val = JsAny.fromNull();
    try std.testing.expect(!isArray(null_val));
}

test "from array" {
    const alloc = std.testing.allocator;

    // Create source array
    var src = try JsAny.newArray(alloc);
    defer src.deinit(alloc);
    try src.arrayPush(alloc, JsAny.fromI64(1));
    try src.arrayPush(alloc, JsAny.fromI64(2));
    try src.arrayPush(alloc, JsAny.fromI64(3));

    // Convert to ArrayList(JsAny)
    var result = try from(alloc, src);
    defer result.deinit(alloc);

    try std.testing.expectEqual(@as(usize, 3), result.items.len);
    try std.testing.expectEqual(@as(i64, 1), result.items[0].asI64());
    try std.testing.expectEqual(@as(i64, 2), result.items[1].asI64());
    try std.testing.expectEqual(@as(i64, 3), result.items[2].asI64());
}

test "from string" {
    const alloc = std.testing.allocator;

    // String "abc" should become ["a", "b", "c"]
    const str = JsAny.fromString("abc");
    var result = try from(alloc, str);
    for (result.items) |*item| item.deinitDeep(alloc);
    result.deinit(alloc);

    // Re-run to verify correctness (no defer so we can check values first)
    var result2 = try from(alloc, str);
    try std.testing.expectEqual(@as(usize, 3), result2.items.len);
    try std.testing.expectEqualStrings("a", result2.items[0].asString(alloc));
    try std.testing.expectEqualStrings("b", result2.items[1].asString(alloc));
    try std.testing.expectEqualStrings("c", result2.items[2].asString(alloc));
    for (result2.items) |*item| item.deinitDeep(alloc);
    result2.deinit(alloc);
}

test "from object with length" {
    const alloc = std.testing.allocator;

    // Create array-like object: {0: "a", 1: "b", length: 2}
    var obj = try JsAny.newObject(alloc);
    defer obj.deinit(alloc);
    try obj.set("0", JsAny.fromString("a"), alloc);
    try obj.set("1", JsAny.fromString("b"), alloc);
    try obj.set("length", JsAny.fromI64(2), alloc);

    var result = try from(alloc, obj);
    defer result.deinit(alloc);

    try std.testing.expectEqual(@as(usize, 2), result.items.len);
    try std.testing.expectEqualStrings("a", result.items[0].asString(alloc));
    try std.testing.expectEqualStrings("b", result.items[1].asString(alloc));
}

test "of" {
    const alloc = std.testing.allocator;

    // Create array from items
    const items = &[_]JsAny{
        JsAny.fromI64(10),
        JsAny.fromString("hello"),
        JsAny.fromBool(true),
    };

    var result = try of(alloc, items);
    defer result.deinit(alloc);

    try std.testing.expectEqual(@as(usize, 3), result.items.len);
    try std.testing.expectEqual(@as(i64, 10), result.items[0].asI64());
    try std.testing.expectEqualStrings("hello", result.items[1].asString(alloc));
    try std.testing.expect(result.items[2].asBool());
}
