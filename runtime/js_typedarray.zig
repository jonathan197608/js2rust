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
        if (OutType == @TypeOf(val)) {
            result[i] = val;
        } else {
            // JS integer wrapping semantics (ToInt8/ToUint8/etc.):
            // @truncate in Zig 0.16.0 requires unsigned input, so we bit-cast
            // to unsigned, truncate the low bits, then bit-cast back to the
            // (possibly signed) target type.
            const SrcUnsigned = std.meta.Int(.unsigned, @bitSizeOf(@TypeOf(val)));
            const DstUnsigned = std.meta.Int(.unsigned, @bitSizeOf(OutType));
            const unsigned_val: SrcUnsigned = @bitCast(val);
            const truncated: DstUnsigned = @truncate(unsigned_val);
            result[i] = @bitCast(truncated);
        }
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
    return convertArray(i32, alloc, arr);
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

/// Accept i64 array and return as []i64 (BigInt64Array — identity copy).
pub fn fromI64AsI64(alloc: Allocator, arr: anytype) ![]i64 {
    return convertArray(i64, alloc, arr);
}

/// Accept i64 array and convert to []u64 (BigUint64Array).
pub fn fromI64AsU64(alloc: Allocator, arr: anytype) ![]u64 {
    return convertArray(u64, alloc, arr);
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

pub fn copyWithinI8(arr: []i8, target: i64, start: i64, end: i64) []i8 {
    const len: i64 = @intCast(arr.len);
    const t = @min(@as(usize, @intCast(if (target < 0) @max(0, len + target) else target)), arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e or t >= arr.len) return arr;

    const count = @min(e - s, arr.len - t);
    std.mem.copyForwards(i8, arr[t..t + count], arr[s..s + count]);
    return arr;
}

pub fn copyWithinU8(arr: []u8, target: i64, start: i64, end: i64) []u8 {
    const len: i64 = @intCast(arr.len);
    const t = @min(@as(usize, @intCast(if (target < 0) @max(0, len + target) else target)), arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e or t >= arr.len) return arr;

    const count = @min(e - s, arr.len - t);
    std.mem.copyForwards(u8, arr[t..t + count], arr[s..s + count]);
    return arr;
}

pub fn copyWithinI32(arr: []i32, target: i64, start: i64, end: i64) []i32 {
    const len: i64 = @intCast(arr.len);
    const t = @min(@as(usize, @intCast(if (target < 0) @max(0, len + target) else target)), arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e or t >= arr.len) return arr;

    const count = @min(e - s, arr.len - t);
    std.mem.copyForwards(i32, arr[t..t + count], arr[s..s + count]);
    return arr;
}

pub fn copyWithinF64(arr: []f64, target: i64, start: i64, end: i64) []f64 {
    const len: i64 = @intCast(arr.len);
    const t = @min(@as(usize, @intCast(if (target < 0) @max(0, len + target) else target)), arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e or t >= arr.len) return arr;

    const count = @min(e - s, arr.len - t);
    std.mem.copyForwards(f64, arr[t..t + count], arr[s..s + count]);
    return arr;
}

// ── fill() — fill array with value, returns arr (JS chaining semantics) ──

pub fn fillI8(arr: []i8, value: i8, start: i64, end: i64) []i8 {
    const len: i64 = @intCast(arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e) return arr;
    @memset(arr[s..e], value);
    return arr;
}

pub fn fillU8(arr: []u8, value: u8, start: i64, end: i64) []u8 {
    const len: i64 = @intCast(arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e) return arr;
    @memset(arr[s..e], value);
    return arr;
}

pub fn fillI32(arr: []i32, value: i32, start: i64, end: i64) []i32 {
    const len: i64 = @intCast(arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e) return arr;
    for (arr[s..e]) |*item| {
        item.* = value;
    }
    return arr;
}

pub fn fillF64(arr: []f64, value: f64, start: i64, end: i64) []f64 {
    const len: i64 = @intCast(arr.len);
    const s = @min(@as(usize, @intCast(if (start < 0) @max(0, len + start) else start)), arr.len);
    const e = if (end == std.math.maxInt(i64))
        arr.len
    else
        @min(@as(usize, @intCast(if (end < 0) @max(0, len + end) else end)), arr.len);

    if (s >= e) return arr;
    for (arr[s..e]) |*item| {
        item.* = value;
    }
    return arr;
}

// ── buffer() — reinterpret slice as underlying byte buffer ──

pub fn bufferU8(arr: []const u8) []const u8 {
    return arr;
}

pub fn bufferI8(arr: []const i8) []const u8 {
    const ptr: [*]const u8 = @ptrCast(arr.ptr);
    return ptr[0 .. arr.len * @sizeOf(i8)];
}

pub fn bufferI16(arr: []const i16) []const u8 {
    const ptr: [*]const u8 = @ptrCast(arr.ptr);
    return ptr[0 .. arr.len * @sizeOf(i16)];
}

pub fn bufferU16(arr: []const u16) []const u8 {
    const ptr: [*]const u8 = @ptrCast(arr.ptr);
    return ptr[0 .. arr.len * @sizeOf(u16)];
}

pub fn bufferI32(arr: []const i32) []const u8 {
    const ptr: [*]const u8 = @ptrCast(arr.ptr);
    return ptr[0 .. arr.len * @sizeOf(i32)];
}

pub fn bufferU32(arr: []const u32) []const u8 {
    const ptr: [*]const u8 = @ptrCast(arr.ptr);
    return ptr[0 .. arr.len * @sizeOf(u32)];
}

pub fn bufferF32(arr: []const f32) []const u8 {
    const ptr: [*]const u8 = @ptrCast(arr.ptr);
    return ptr[0 .. arr.len * @sizeOf(f32)];
}

pub fn bufferF64(arr: []const f64) []const u8 {
    const ptr: [*]const u8 = @ptrCast(arr.ptr);
    return ptr[0 .. arr.len * @sizeOf(f64)];
}

// ── byteLength() — size in bytes of the typed array ──
// Uses std.math.cast to safely convert usize → i64, avoiding @intCast
// panic on 64-bit platforms with arrays exceeding i64 range (P1-6).

pub fn byteLengthI8(arr: []const i8) i64 {
    return std.math.cast(i64, arr.len * @sizeOf(i8)) orelse std.math.maxInt(i64);
}

pub fn byteLengthU8(arr: []const u8) i64 {
    return std.math.cast(i64, arr.len * @sizeOf(u8)) orelse std.math.maxInt(i64);
}

pub fn byteLengthI16(arr: []const i16) i64 {
    return std.math.cast(i64, arr.len * @sizeOf(i16)) orelse std.math.maxInt(i64);
}

pub fn byteLengthU16(arr: []const u16) i64 {
    return std.math.cast(i64, arr.len * @sizeOf(u16)) orelse std.math.maxInt(i64);
}

pub fn byteLengthI32(arr: []const i32) i64 {
    return std.math.cast(i64, arr.len * @sizeOf(i32)) orelse std.math.maxInt(i64);
}

pub fn byteLengthU32(arr: []const u32) i64 {
    return std.math.cast(i64, arr.len * @sizeOf(u32)) orelse std.math.maxInt(i64);
}

pub fn byteLengthF32(arr: []const f32) i64 {
    return std.math.cast(i64, arr.len * @sizeOf(f32)) orelse std.math.maxInt(i64);
}

pub fn byteLengthF64(arr: []const f64) i64 {
    return std.math.cast(i64, arr.len * @sizeOf(f64)) orelse std.math.maxInt(i64);
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
    _ = copyWithinI32(&arr, 0, 3, 5);
    try std.testing.expectEqual(@as(i32, 4), arr[0]);
    try std.testing.expectEqual(@as(i32, 5), arr[1]);
    try std.testing.expectEqual(@as(i32, 3), arr[2]);
}

test "fillI32" {
    var arr = [_]i32{ 1, 2, 3, 4, 5 };
    _ = fillI32(&arr, 0, 1, 4);
    try std.testing.expectEqual(@as(i32, 1), arr[0]);
    try std.testing.expectEqual(@as(i32, 0), arr[1]);
    try std.testing.expectEqual(@as(i32, 0), arr[3]);
    try std.testing.expectEqual(@as(i32, 5), arr[4]);
}

// ── Integer wrapping tests (RT-1: @intCast to @truncate) ──

test "fromI64AsI32: i64 to i32 wrapping (ToInt32 semantics)" {
    const alloc = std.testing.allocator;
    const input = [_]i64{ 0, 1, -1, 2147483647, -2147483648, 2147483648, 4294967296 };
    const result = try fromI64AsI32(alloc, &input);
    defer alloc.free(result);
    try std.testing.expectEqual(@as(i32, 0), result[0]);
    try std.testing.expectEqual(@as(i32, 1), result[1]);
    try std.testing.expectEqual(@as(i32, -1), result[2]);
    try std.testing.expectEqual(@as(i32, 2147483647), result[3]);
    try std.testing.expectEqual(@as(i32, -2147483648), result[4]);
    // 2147483648 wraps to -2147483648
    try std.testing.expectEqual(@as(i32, -2147483648), result[5]);
    // 4294967296 wraps to 0
    try std.testing.expectEqual(@as(i32, 0), result[6]);
}

test "fromI64AsU8: i64 to u8 wrapping (ToUint8 semantics)" {
    const alloc = std.testing.allocator;
    const input = [_]i64{ 0, 255, 256, 257, -1 };
    const result = try fromI64AsU8(alloc, &input);
    defer alloc.free(result);
    try std.testing.expectEqual(@as(u8, 0), result[0]);
    try std.testing.expectEqual(@as(u8, 255), result[1]);
    // 256 wraps to 0
    try std.testing.expectEqual(@as(u8, 0), result[2]);
    // 257 wraps to 1
    try std.testing.expectEqual(@as(u8, 1), result[3]);
    // -1 wraps to 255 (unsigned)
    try std.testing.expectEqual(@as(u8, 255), result[4]);
}

test "fromI64AsI8: i64 to i8 wrapping (ToInt8 semantics)" {
    const alloc = std.testing.allocator;
    const input = [_]i64{ 0, 127, 128, -128, -129, 256, -256 };
    const result = try fromI64AsI8(alloc, &input);
    defer alloc.free(result);
    try std.testing.expectEqual(@as(i8, 0), result[0]);
    try std.testing.expectEqual(@as(i8, 127), result[1]);
    // 128 wraps to -128
    try std.testing.expectEqual(@as(i8, -128), result[2]);
    try std.testing.expectEqual(@as(i8, -128), result[3]);
    // -129 wraps to 127
    try std.testing.expectEqual(@as(i8, 127), result[4]);
    // 256 wraps to 0
    try std.testing.expectEqual(@as(i8, 0), result[5]);
    // -256 wraps to 0
    try std.testing.expectEqual(@as(i8, 0), result[6]);
}
