//! JS Array method implementations for Zig.
//! Works with []const i64 slices.
//! All allocating functions take `alloc: std.mem.Allocator` as first parameter.

const std = @import("std");
const Allocator = std.mem.Allocator;

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
