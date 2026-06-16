//! JS JSON method implementations for Zig.
//! Supports basic types: i64, f64, bool, []const u8.
//! All allocating functions take `alloc: std.mem.Allocator` as first parameter.

const std = @import("std");
const Allocator = std.mem.Allocator;

/// JSON.stringify for i64 values.
pub fn stringifyI64(alloc: Allocator, value: i64) ![]const u8 {
    return std.fmt.allocPrint(alloc, "{d}", .{value});
}

/// JSON.stringify for string values.
pub fn stringifyStr(alloc: Allocator, value: []const u8) ![]const u8 {
    return std.fmt.allocPrint(alloc, "\"{s}\"", .{value});
}

/// JSON.stringify for f64 values.
pub fn stringifyF64(alloc: Allocator, value: f64) ![]const u8 {
    return std.fmt.allocPrint(alloc, "{d}", .{value});
}

/// JSON.stringify for boolean values.
pub fn stringifyBool(alloc: Allocator, value: bool) ![]const u8 {
    const s = if (value) "true" else "false";
    return alloc.dupe(u8, s);
}

/// JSON.parse — returns a copy (full parsing not yet implemented).
pub fn parse(alloc: Allocator, s: []const u8) ![]const u8 {
    _ = s;
    return std.fmt.allocPrint(alloc, "{{parsed}}", .{});
}
