//! JS Number static method implementations for Zig.
//! Operates on numeric values directly, no allocation needed.

const std = @import("std");

/// Number.isNaN — check if a value is NaN.
pub fn isNaN(val: f64) bool {
    return std.math.isNan(val);
}

/// Number.isFinite — check if a value is finite.
pub fn isFinite(val: f64) bool {
    return !std.math.isInf(val) and !std.math.isNan(val);
}

/// Number.isInteger — check if a value is an integer (safe within i64 range).
pub fn isInteger(val: f64) bool {
    if (std.math.isNan(val) or std.math.isInf(val)) return false;
    return @mod(val, 1.0) == 0.0;
}

/// Number.parseInt — parse an integer from a string.
pub fn parseInt(s: []const u8) i64 {
    return std.fmt.parseInt(i64, s, 10) catch 0;
}

/// Number.parseFloat — parse a float from a string.
pub fn parseFloat(s: []const u8) f64 {
    return std.fmt.parseFloat(f64, s) catch 0.0;
}

/// Number.isSafeInteger — check if a value is a safe integer (|v| <= 2^53-1).
pub fn isSafeInteger(val: f64) bool {
    if (std.math.isNan(val) or std.math.isInf(val)) return false;
    if (@mod(val, 1.0) != 0.0) return false;
    const v: i64 = @intFromFloat(val);
    return v >= -9007199254740991 and v <= 9007199254740991;
}

/// Number.prototype.toFixed — format a float to fixed-point string.
/// Uses inline for + comptimePrint to generate variable-precision format string.
pub fn toFixed(alloc: std.mem.Allocator, val: f64, digits: i64) ![]const u8 {
    // Handle special values
    if (std.math.isNan(val)) return alloc.dupe(u8, "NaN");
    if (std.math.isInf(val)) {
        return if (val > 0) alloc.dupe(u8, "Infinity") else alloc.dupe(u8, "-Infinity");
    }
    const d: usize = @intCast(@max(0, @min(100, digits)));

    var buf: [128]u8 = undefined;
    // Use inline for to generate all precision cases at comptime
    inline for (0..21) |p| {
        if (d == p) {
            const format = comptime std.fmt.comptimePrint("{{d:.{d}}}", .{p});
            const s = try std.fmt.bufPrint(&buf, format, .{val});
            return alloc.dupe(u8, s);
        }
    }
    // Fallback for digits 21-100: use precision 6
    const s = try std.fmt.bufPrint(&buf, "{d:.6}", .{val});
    return alloc.dupe(u8, s);
}

// ── Tests ──

test "isNaN" {
    try std.testing.expect(isNaN(std.math.nan(f64)));
    try std.testing.expect(!isNaN(42.0));
}

test "isFinite" {
    try std.testing.expect(isFinite(42.0));
    try std.testing.expect(!isFinite(std.math.inf(f64)));
    try std.testing.expect(!isFinite(std.math.nan(f64)));
}

test "isInteger" {
    try std.testing.expect(isInteger(42.0));
    try std.testing.expect(isInteger(-7.0));
    try std.testing.expect(!isInteger(3.14));
    try std.testing.expect(!isInteger(std.math.nan(f64)));
}

test "parseInt" {
    try std.testing.expectEqual(@as(i64, 42), parseInt("42"));
    try std.testing.expectEqual(@as(i64, 0), parseInt("abc"));
}

test "parseFloat" {
    try std.testing.expectEqual(@as(f64, 3.14), parseFloat("3.14"));
    try std.testing.expectEqual(@as(f64, 0.0), parseFloat("abc"));
}

test "isSafeInteger" {
    try std.testing.expect(isSafeInteger(42.0));
    try std.testing.expect(isSafeInteger(@as(f64, 9007199254740991)));
    try std.testing.expect(isSafeInteger(@as(f64, -9007199254740991)));
    try std.testing.expect(!isSafeInteger(@as(f64, 9007199254740992)));
    try std.testing.expect(!isSafeInteger(@as(f64, -9007199254740992)));
    try std.testing.expect(!isSafeInteger(3.14));
    try std.testing.expect(!isSafeInteger(std.math.nan(f64)));
    try std.testing.expect(!isSafeInteger(std.math.inf(f64)));
}

test "toFixed" {
    const a = std.testing.allocator;
    const r1 = try toFixed(a, 3.14159, 2);
    defer a.free(r1);
    try std.testing.expectEqualStrings("3.14", r1);

    const r2 = try toFixed(a, 3.0, 3);
    defer a.free(r2);
    try std.testing.expectEqualStrings("3.000", r2);

    const r3 = try toFixed(a, -2.5, 0);
    defer a.free(r3);
    try std.testing.expectEqualStrings("-3", r3);

    const r4 = try toFixed(a, std.math.nan(f64), 2);
    defer a.free(r4);
    try std.testing.expectEqualStrings("NaN", r4);

    const r5 = try toFixed(a, std.math.inf(f64), 2);
    defer a.free(r5);
    try std.testing.expectEqualStrings("Infinity", r5);
}
