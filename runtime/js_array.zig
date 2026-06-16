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

/// Array.pop — remove and return last element.
pub fn pop(arr: []const i64) ?i64 {
    if (arr.len == 0) return null;
    return arr[arr.len - 1];
}

/// Array.join — join elements with separator, returns new string.
pub fn join(alloc: Allocator, arr: []const i64, sep: []const u8) ![]const u8 {
    if (arr.len == 0) return &[0]u8{};

    var buf = std.ArrayList(u8).init(alloc);
    errdefer buf.deinit();
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
    var buf = std.ArrayList(i64).init(alloc);
    errdefer buf.deinit();
    for (arr) |val| {
        if (val > threshold) {
            try buf.append(val);
        }
    }
    return buf.toOwnedSlice();
}
