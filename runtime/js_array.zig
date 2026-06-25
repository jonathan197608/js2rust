//! JS Array method implementations for Zig.
//! Works with []const i64 slices.
//! All allocating functions take `alloc: std.mem.Allocator` as first parameter.

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsAny = @import("jsany.zig").JsAny;

/// Array.isArray — always true for Zig arrays (type system guarantees).
pub fn isArray(_: anytype) bool {
    return true;
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

/// Array.join — join elements with separator, returns new string.
pub fn join(alloc: Allocator, arr: []const i64, sep: []const u8) ![]const u8 {
    if (arr.len == 0) return &[0]u8{};

    var buf: std.ArrayList(u8) = .empty;
    errdefer buf.deinit(alloc);
    var writer = buf.writer();

    for (arr, 0..) |val, i| {
        if (i > 0) try writer.writeAll(sep);
        try writer.print("{d}", .{val});
    }

    return buf.toOwnedSlice();
}

/// Array.map — simplified: multiply each element by a scalar.
pub fn map(alloc: Allocator, arr: []const i64, mul: i64) ![]const i64 {
    const result = try alloc.alloc(i64, arr.len);
    for (arr, 0..) |val, i| {
        result[i] = val * mul;
    }
    return result;
}

/// Array.filter — keep elements above threshold.
pub fn filter(alloc: Allocator, arr: []const i64, threshold: i64) ![]const i64 {
    var buf: std.ArrayList(i64) = .empty;
    errdefer buf.deinit(alloc);
    for (arr) |val| {
        if (val > threshold) {
            try buf.append(alloc, val);
        }
    }
    return buf.toOwnedSlice();
}

/// Array.indexOf — find first index of val, or -1.
pub fn indexOf(arr: []const i64, val: i64) i64 {
    for (arr, 0..) |v, i| {
        if (v == val) return @intCast(i);
    }
    return -1;
}

/// Array.includes — check if arr contains val.
pub fn includes(arr: []const i64, val: i64) bool {
    return indexOf(arr, val) != -1;
}

/// Array.reverse — return new reversed slice.
pub fn reverse(alloc: Allocator, arr: []const i64) ![]const i64 {
    const result = try alloc.alloc(i64, arr.len);
    for (arr, 0..) |_, i| {
        result[i] = arr[arr.len - 1 - i];
    }
    return result;
}

/// Array.slice — extract sub-slice (start inclusive, end exclusive).
/// Returns borrowed slice. Negative indices count from end.
pub fn slice(arr: []const i64, start: i64, end: i64) []const i64 {
    const len: i64 = @intCast(arr.len);
    var st: i64 = start;
    var en: i64 = end;

    if (st < 0) st = @max(0, len + st);
    if (en < 0) en = @max(0, len + en);

    st = @min(@max(0, st), len);
    en = @min(@max(0, en), len);
    if (st >= en) return &[0]i64{};

    return arr[@intCast(st)..@intCast(en)];
}

/// Array.concat — concatenate two arrays, returns new slice.
pub fn concat(alloc: Allocator, a: []const i64, b: []const i64) ![]const i64 {
    const result = try alloc.alloc(i64, a.len + b.len);
    @memcpy(result[0..a.len], a);
    @memcpy(result[a.len..], b);
    return result;
}

/// Array.sort — return new ascending-sorted slice.
pub fn sort(alloc: Allocator, arr: []const i64) ![]const i64 {
    const result = try alloc.dupe(i64, arr);
    std.mem.sort(i64, result, {}, comptime std.sort.asc(i64));
    return result;
}

/// Array.flat — identity operation for i64 arrays (elements are already flat).
pub fn flat(alloc: Allocator, arr: []const i64) ![]const i64 {
    return try alloc.dupe(i64, arr);
}

/// Array.flatMap — same as identity map (simplified).
/// For i64 arrays, returns a copy of the original array.
pub fn flatMap(alloc: Allocator, arr: []const i64, _mul: i64) ![]const i64 {
    _ = _mul;
    return try alloc.dupe(i64, arr);
}

// ── Tests ──

test "indexOf" {
    try std.testing.expectEqual(@as(i64, 2), indexOf(&[_]i64{ 10, 20, 30, 40 }, 30));
    try std.testing.expectEqual(@as(i64, -1), indexOf(&[_]i64{ 10, 20 }, 99));
}

test "includes" {
    try std.testing.expect(includes(&[_]i64{ 10, 20, 30 }, 20));
    try std.testing.expect(!includes(&[_]i64{ 10, 20 }, 99));
}

test "reverse" {
    const result = try reverse(std.testing.allocator, &[_]i64{ 1, 2, 3 });
    defer std.testing.allocator.free(result);
    try std.testing.expectEqual(@as(i64, 3), result[0]);
    try std.testing.expectEqual(@as(i64, 2), result[1]);
    try std.testing.expectEqual(@as(i64, 1), result[2]);
}

test "slice" {
    const arr = &[_]i64{ 10, 20, 30, 40, 50 };
    const s = slice(arr, 1, 4);
    try std.testing.expectEqual(@as(usize, 3), s.len);
    try std.testing.expectEqual(@as(i64, 20), s[0]);
    try std.testing.expectEqual(@as(i64, 40), s[2]);
}

test "concat" {
    const a = &[_]i64{ 1, 2 };
    const b = &[_]i64{ 3, 4, 5 };
    const result = try concat(std.testing.allocator, a, b);
    defer std.testing.allocator.free(result);
    try std.testing.expectEqual(@as(usize, 5), result.len);
    try std.testing.expectEqual(@as(i64, 3), result[2]);
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

// ── ArrayList(JsAny) helpers (for dynamic arrays) ─────────────

/// In-place reverse of ArrayList(JsAny).
/// Used by arr.reverse() for dynamic arrays.
/// Returns void (array is mutated in place).
pub fn reverseInPlace(arr: *std.ArrayList(JsAny)) void {
    std.mem.reverse(JsAny, arr.items);
}

/// In-place sort of ArrayList(JsAny) using JsAny.lt() comparator.
/// Used by arr.sort() for dynamic arrays.
/// Returns void (array is mutated in place).
pub fn sortInPlace(arr: *std.ArrayList(JsAny)) void {
    std.mem.sort(JsAny, arr.items, {}, struct {
        fn lessThan(_: void, a: JsAny, b: JsAny) bool {
            return a.lt(b);
        }
    }.lessThan);
}

/// Slice ArrayList(JsAny) and return new ArrayList(JsAny).
/// Negative indices count from end.
pub fn sliceAny(alloc: Allocator, arr: *const std.ArrayList(JsAny), start: i64, end: i64) !std.ArrayList(JsAny) {
    const len: i64 = @intCast(arr.items.len);
    var st: i64 = start;
    var en: i64 = end;

    if (st < 0) st = @max(0, len + st);
    if (en < 0) en = @max(0, len + en);

    st = @min(@max(0, st), len);
    en = @min(@max(0, en), len);
    if (st >= en) return std.ArrayList(JsAny).empty;

    var result = std.ArrayList(JsAny).empty;
    errdefer result.deinit(alloc);
    try result.ensureTotalCapacity(alloc, @intCast(en - st));
    for (arr.items[@intCast(st)..@intCast(en)]) |item| {
        result.appendAssumeCapacity(item);
    }
    return result;
}

/// Join ArrayList(JsAny) elements with separator, returns allocated string.
pub fn joinAny(alloc: Allocator, arr: *const std.ArrayList(JsAny), sep: []const u8) ![]const u8 {
    if (arr.items.len == 0) return &[0]u8{};

    var buf = std.ArrayList(u8).empty;
    errdefer buf.deinit(alloc);
    try buf.ensureTotalCapacity(alloc, arr.items.len * 4); // estimate: 4 chars per element
    var writer = buf.writer();

    for (arr.items, 0..) |item, i| {
        if (i > 0) try writer.writeAll(sep);
        try writer.print("{f}", .{item});
    }

    return buf.toOwnedSlice(alloc);
}


/// Map ArrayList(JsAny) by multiplying each element by scalar.
/// Returns new ArrayList(JsAny).
pub fn mapAnyScalar(alloc: Allocator, arr: *const std.ArrayList(JsAny), scalar: i64) !std.ArrayList(JsAny) {
    var result = std.ArrayList(JsAny).empty;
    errdefer result.deinit(alloc);
    try result.ensureTotalCapacity(alloc, arr.items.len);
    for (arr.items) |item| {
        const val = item.asI64();
        result.appendAssumeCapacity(JsAny.fromI64(val * scalar));
    }
    return result;
}

/// Filter ArrayList(JsAny), keeping elements > threshold.
/// Returns new ArrayList(JsAny).
pub fn filterAnyThreshold(alloc: Allocator, arr: *const std.ArrayList(JsAny), threshold: i64) !std.ArrayList(JsAny) {
    var result = std.ArrayList(JsAny).empty;
    errdefer result.deinit(alloc);
    for (arr.items) |item| {
        const val = item.asI64();
        if (val > threshold) {
            try result.append(alloc, item);
        }
    }
    return result;
}

/// Map ArrayList(JsAny) using a comptime function.
/// Returns new ArrayList(JsAny).
pub fn mapWithFn(
    alloc: Allocator,
    arr: *const std.ArrayList(JsAny),
    comptime f: fn(JsAny) JsAny,
) !std.ArrayList(JsAny) {
    var result = std.ArrayList(JsAny).empty;
    errdefer result.deinit(alloc);
    try result.ensureTotalCapacity(alloc, arr.items.len);
    for (arr.items) |item| {
        result.appendAssumeCapacity(f(item));
    }
    return result;
}

/// Filter ArrayList(JsAny) using a comptime predicate.
/// Returns new ArrayList(JsAny).
pub fn filterWithFn(
    alloc: Allocator,
    arr: *const std.ArrayList(JsAny),
    comptime pred: fn(JsAny) bool,
) !std.ArrayList(JsAny) {
    var result = std.ArrayList(JsAny).empty;
    errdefer result.deinit(alloc);
    try result.ensureTotalCapacity(alloc, arr.items.len);
    for (arr.items) |item| {
        if (pred(item)) {
            result.appendAssumeCapacity(item);
        }
    }
    return result;
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

test "flat" {
    const result = try flat(std.testing.allocator, &[_]i64{ 1, 2, 3 });
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualSlices(i64, &[_]i64{ 1, 2, 3 }, result);
}

test "flatMap" {
    const result = try flatMap(std.testing.allocator, &[_]i64{ 1, 2, 3 }, 2);
    defer std.testing.allocator.free(result);
    // flatMap for i64 is identity (returns copy of original)
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
