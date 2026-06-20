//! TypedArray implementations for Zig.
//! Maps JavaScript TypedArray types to Zig slices.
//!
//! TypedArray → Zig slice mapping:
//!   Int8Array    → []i8
//!   Uint8Array   → []u8
//!   Uint8ClampedArray → []u8
//!   Int16Array   → []i16
//!   Uint16Array  → []u16
//!   Int32Array   → []i32
//!   Uint32Array  → []u32
//!   Float32Array → []f32
//!   Float64Array → []f64

const std = @import("std");
const Allocator = std.mem.Allocator;

// ── Generic helper: convert array/slice of any numeric type to []OutType ──

fn convertArray(comptime OutType: type, alloc: Allocator, arr: anytype) ![]OutType {
    const T = @TypeOf(arr);
    const slice = if (@typeInfo(T) == .array)
        arr[0..]
    else
        arr;
    const result = try alloc.alloc(OutType, slice.len);
    for (slice, 0..) |val, i| {
        result[i] = @intCast(val);
    }
    return result;
}

// ── Helper: convert JS-style index (negative = from end) ──

fn adjustIndex(idx: i64, len: usize) usize {
    const idx_i64: i64 = if (idx < 0)
        @max(0, @as(i64, @intCast(len)) + idx)
    else
        @min(@as(i64, @intCast(len)), idx);
    return @intCast(idx_i64);
}

// ── from() — convert slice to []T ──
// In JS: Int32Array.from([1, 2, 3]) creates a new Int32Array
// In Zig: we just return the slice (type system handles it)

pub fn fromI8(alloc: Allocator, arr: []const i8) ![]i8 {
    return try alloc.dupe(i8, arr);
}

pub fn fromU8(alloc: Allocator, arr: []const u8) ![]u8 {
    return try alloc.dupe(u8, arr);
}

pub fn fromI16(alloc: Allocator, arr: []const i16) ![]i16 {
    return try alloc.dupe(i16, arr);
}

pub fn fromU16(alloc: Allocator, arr: []const u16) ![]u16 {
    return try alloc.dupe(u16, arr);
}

pub fn fromI32(alloc: Allocator, arr: []const i32) ![]i32 {
    return try alloc.dupe(i32, arr);
}

/// Accept i64 array and convert to []i32.
/// Used by code generator which generates i64 arrays for JS number literals.
pub fn fromI64AsI32(alloc: Allocator, arr: anytype) ![]i32 {
    const T = @TypeOf(arr);
    const slice = if (@typeInfo(T) == .array)
        arr[0..]
    else
        arr;
    const result = try alloc.alloc(i32, slice.len);
    for (slice, 0..) |val, i| {
        result[i] = @intCast(val);
    }
    return result;
}

pub fn fromI64AsI8(alloc: Allocator, arr: anytype) ![]i8 {
    return convertArray(i8, alloc, arr);
}

pub fn fromI64AsU8(alloc: Allocator, arr: anytype) ![]u8 {
    return convertArray(u8, alloc, arr);
}

pub fn fromI64AsI16(alloc: Allocator, arr: anytype) ![]i16 {
    return convertArray(i16, alloc, arr);
}

pub fn fromI64AsU16(alloc: Allocator, arr: anytype) ![]u16 {
    return convertArray(u16, alloc, arr);
}

pub fn fromI64AsU32(alloc: Allocator, arr: anytype) ![]u32 {
    return convertArray(u32, alloc, arr);
}

pub fn fromF64AsF32(alloc: Allocator, arr: anytype) ![]f32 {
    const T = @TypeOf(arr);
    const slice = if (@typeInfo(T) == .array)
        arr[0..]
    else
        arr;
    const result = try alloc.alloc(f32, slice.len);
    for (slice, 0..) |val, i| {
        result[i] = @floatCast(val);
    }
    return result;
}

pub fn fromU32(alloc: Allocator, arr: []const u32) ![]u32 {
    return try alloc.dupe(u32, arr);
}

pub fn fromF32(alloc: Allocator, arr: []const f32) ![]f32 {
    return try alloc.dupe(f32, arr);
}

pub fn fromF64(alloc: Allocator, arr: []const f64) ![]f64 {
    return try alloc.dupe(f64, arr);
}

// ── Empty slice helpers (used in catch branches) ──

pub fn emptyI8() []i8 {
    return &[_]i8{};
}

pub fn emptyU8() []u8 {
    return &[_]u8{};
}

pub fn emptyI16() []i16 {
    return &[_]i16{};
}

pub fn emptyU16() []u16 {
    return &[_]u16{};
}

pub fn emptyI32() []i32 {
    return &[_]i32{};
}

pub fn emptyU32() []u32 {
    return &[_]u32{};
}

pub fn emptyF32() []f32 {
    return &[_]f32{};
}

pub fn emptyF64() []f64 {
    return &[_]f64{};
}

// ── of() — create TypedArray from variadic args ──
// JS: Int32Array.of(1, 2, 3)
// Zig: we just create a slice literal

pub fn ofI8(alloc: Allocator, items: []const i8) ![]i8 {
    return try alloc.dupe(i8, items);
}

pub fn ofU8(alloc: Allocator, items: []const u8) ![]u8 {
    return try alloc.dupe(u8, items);
}

pub fn ofI32(alloc: Allocator, items: []const i32) ![]i32 {
    return try alloc.dupe(i32, items);
}

pub fn ofF64(alloc: Allocator, items: []const f64) ![]f64 {
    return try alloc.dupe(f64, items);
}

// ── get() — access element at index ──

pub fn getI8(arr: []const i8, index: i64) ?i8 {
    const idx = adjustIndex(index, arr.len);
    if (idx >= arr.len) return null;
    return arr[idx];
}

pub fn getU8(arr: []const u8, index: i64) ?u8 {
    const idx = adjustIndex(index, arr.len);
    if (idx >= arr.len) return null;
    return arr[idx];
}

pub fn getI32(arr: []const i32, index: i64) ?i32 {
    const idx = adjustIndex(index, arr.len);
    if (idx >= arr.len) return null;
    return arr[idx];
}

pub fn getF64(arr: []const f64, index: i64) ?f64 {
    const idx = adjustIndex(index, arr.len);
    if (idx >= arr.len) return null;
    return arr[idx];
}

// ── set() — set element at index ──
// Returns true if successful, false if out of bounds
// Note: Zig slices are immutable ([]const T), so we use []T for set operations

pub fn setI8(arr: []i8, index: i64, value: i8) bool {
    const idx = adjustIndex(index, arr.len);
    if (idx >= arr.len) return false;
    arr[idx] = value;
    return true;
}

pub fn setU8(arr: []u8, index: i64, value: u8) bool {
    const idx = adjustIndex(index, arr.len);
    if (idx >= arr.len) return false;
    arr[idx] = value;
    return true;
}

pub fn setI32(arr: []i32, index: i64, value: i32) bool {
    const idx = adjustIndex(index, arr.len);
    if (idx >= arr.len) return false;
    arr[idx] = value;
    return true;
}

pub fn setF64(arr: []f64, index: i64, value: f64) bool {
    const idx = adjustIndex(index, arr.len);
    if (idx >= arr.len) return false;
    arr[idx] = value;
    return true;
}

// ── slice() — extract sub-slice (borrowed) ──

pub fn sliceI8(arr: []const i8, start: i64, end: i64) []const i8 {
    const st = adjustIndex(start, arr.len);
    const en = if (end == std.math.maxInt(i64))
        arr.len
    else
        adjustIndex(end, arr.len);
    if (st >= en) return &[0]i8{};
    return arr[st..en];
}

pub fn sliceU8(arr: []const u8, start: i64, end: i64) []const u8 {
    const st = adjustIndex(start, arr.len);
    const en = if (end == std.math.maxInt(i64))
        arr.len
    else
        adjustIndex(end, arr.len);
    if (st >= en) return &[0]u8{};
    return arr[st..en];
}

pub fn sliceI32(arr: []const i32, start: i64, end: i64) []const i32 {
    const st = adjustIndex(start, arr.len);
    const en = if (end == std.math.maxInt(i64))
        arr.len
    else
        adjustIndex(end, arr.len);
    if (st >= en) return &[0]i32{};
    return arr[st..en];
}

pub fn sliceF64(arr: []const f64, start: i64, end: i64) []const f64 {
    const st = adjustIndex(start, arr.len);
    const en = if (end == std.math.maxInt(i64))
        arr.len
    else
        adjustIndex(end, arr.len);
    if (st >= en) return &[0]f64{};
    return arr[st..en];
}

// ── subarray() — alias for slice() ──

pub fn subarrayI8(arr: []const i8, start: i64, end: i64) []const i8 {
    return sliceI8(arr, start, end);
}

pub fn subarrayU8(arr: []const u8, start: i64, end: i64) []const u8 {
    return sliceU8(arr, start, end);
}

pub fn subarrayI32(arr: []const i32, start: i64, end: i64) []const i32 {
    return sliceI32(arr, start, end);
}

pub fn subarrayF64(arr: []const f64, start: i64, end: i64) []const f64 {
    return sliceF64(arr, start, end);
}

// ── copyWithin() — copy sequence within array ──

pub fn copyWithinI8(arr: []i8, target: i64, start: i64, end: i64) void {
    const len: i64 = @intCast(arr.len);
    const t = @min(@as(usize, @intCast(if (target < 0) @max(0, len + target) else target)), arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e or t >= arr.len) return;

    const count = @min(e - s, arr.len - t);
    std.mem.copyForwards(i8, arr[t..t + count], arr[s..s + count]);
}

pub fn copyWithinU8(arr: []u8, target: i64, start: i64, end: i64) void {
    const len: i64 = @intCast(arr.len);
    const t = @min(@as(usize, @intCast(if (target < 0) @max(0, len + target) else target)), arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e or t >= arr.len) return;

    const count = @min(e - s, arr.len - t);
    std.mem.copyForwards(u8, arr[t..t + count], arr[s..s + count]);
}

pub fn copyWithinI32(arr: []i32, target: i64, start: i64, end: i64) void {
    const len: i64 = @intCast(arr.len);
    const t = @min(@as(usize, @intCast(if (target < 0) @max(0, len + target) else target)), arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e or t >= arr.len) return;

    const count = @min(e - s, arr.len - t);
    std.mem.copyForwards(i32, arr[t..t + count], arr[s..s + count]);
}

// ── fill() — fill array with value ──

pub fn fillI8(arr: []i8, value: i8, start: i64, end: i64) void {
    const len: i64 = @intCast(arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e) return;
    @memset(arr[s..e], value);
}

pub fn fillU8(arr: []u8, value: u8, start: i64, end: i64) void {
    const len: i64 = @intCast(arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e) return;
    @memset(arr[s..e], value);
}

pub fn fillI32(arr: []i32, value: i32, start: i64, end: i64) void {
    const len: i64 = @intCast(arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e) return;
    for (arr[s..e]) |*item| {
        item.* = value;
    }
}

// ── Tests ──

test "getI32" {
    const arr = &[_]i32{ 10, 20, 30 };
    try std.testing.expectEqual(@as(?i32, 10), getI32(arr, 0));
    try std.testing.expectEqual(@as(?i32, 30), getI32(arr, 2));
    try std.testing.expectEqual(@as(?i32, null), getI32(arr, 99));
}

test "setI32" {
    var arr = [_]i32{ 10, 20, 30 };
    try std.testing.expect(setI32(&arr, 1, 99));
    try std.testing.expectEqual(@as(i32, 99), arr[1]);
    try std.testing.expect(!setI32(&arr, 99, 0));
}

test "sliceI32" {
    const arr = &[_]i32{ 10, 20, 30, 40, 50 };
    const s = sliceI32(arr, 1, 4);
    try std.testing.expectEqual(@as(usize, 3), s.len);
    try std.testing.expectEqual(@as(i32, 20), s[0]);
    try std.testing.expectEqual(@as(i32, 40), s[2]);
}

test "copyWithinI32" {
    var arr = [_]i32{ 1, 2, 3, 4, 5 };
    copyWithinI32(&arr, 0, 3, 5);
    try std.testing.expectEqual(@as(i32, 4), arr[0]);
    try std.testing.expectEqual(@as(i32, 5), arr[1]);
    try std.testing.expectEqual(@as(i32, 3), arr[2]);
}

test "fillI32" {
    var arr = [_]i32{ 1, 2, 3, 4, 5 };
    fillI32(&arr, 0, 1, 4);
    try std.testing.expectEqual(@as(i32, 1), arr[0]);
    try std.testing.expectEqual(@as(i32, 0), arr[1]);
    try std.testing.expectEqual(@as(i32, 0), arr[3]);
    try std.testing.expectEqual(@as(i32, 5), arr[4]);
}
