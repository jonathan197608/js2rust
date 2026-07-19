//! JsValue — JS-style dynamically-typed value for Zig
//! Used when an object needs dynamic property access (computed with variable key).
//! Objects that are only accessed with static keys use generated struct types instead.

const std = @import("std");

pub const Allocator = std.mem.Allocator;

/// Tag for the dynamic value type.
pub const JsValue = union(enum) {
    int: i64,
    float: f64,
    bool: bool,
    string: []const u8,
    null: void,
    undefined: void,

    // --- constructors ---

    pub fn fromI64(v: i64) JsValue {
        return .{ .int = v };
    }

    pub fn fromF64(v: f64) JsValue {
        return .{ .float = v };
    }

    pub fn fromBool(v: bool) JsValue {
        return .{ .bool = v };
    }

    pub fn fromString(v: []const u8) JsValue {
        return .{ .string = v };
    }

    pub fn fromNull() JsValue {
        return .{ .null = {} };
    }

    // --- type checks ---

    pub fn isInt(self: JsValue) bool {
        return self == .int;
    }

    pub fn isFloat(self: JsValue) bool {
        return self == .float;
    }

    pub fn isString(self: JsValue) bool {
        return self == .string;
    }

    pub fn isNull(self: JsValue) bool {
        return self == .null;
    }

    pub fn isUndefined(self: JsValue) bool {
        return self == .undefined;
    }

    pub fn isNumber(self: JsValue) bool {
        return self == .int or self == .float;
    }

    // --- std.fmt integration (for template literals) ---

    /// Custom formatter so `std.fmt.allocPrint("{}", .{jsvalue})` outputs the JS string representation.
    pub fn format(self: JsValue, writer: *std.Io.Writer) std.Io.Writer.Error!void {
        switch (self) {
            .int => |v| try writer.print("{d}", .{v}),
            .float => |v| try writer.print("{d}", .{v}),
            .bool => |v| try writer.writeAll(if (v) "true" else "false"),
            .string => |s| try writer.writeAll(s),
            .null => try writer.writeAll("null"),
            .undefined => try writer.writeAll("undefined"),
        }
    }

    // --- coercion helpers (JS semantics) ---

    pub fn asI64(self: JsValue) i64 {
        return switch (self) {
            .int => |v| v,
            // JS spec: ToInt32 / ToInteger on NaN or ±Inf yields 0; values
            // outside the safe-i64 range also coerce to 0. Zig's
            // @intFromFloat panics on NaN/Inf/out-of-range in safe mode, so
            // we guard those cases first.
            .float => |v| blk: {
                if (std.math.isNan(v) or std.math.isInf(v)) break :blk 0;
                // i64 range: [-9223372036854775808, 9223372036854775807]
                // ≈ [-9.223372036854776e18, 9.223372036854776e18] in f64.
                // Use the representable f64 bounds; values outside return 0.
                const max_i64_f: f64 = 9.223372036854776e18;
                const min_i64_f: f64 = -9.223372036854776e18;
                if (v > max_i64_f or v < min_i64_f) break :blk 0;
                break :blk @as(i64, @intFromFloat(v));
            },
            .bool => |v| if (v) 1 else 0,
            .string => |v| std.fmt.parseInt(i64, v, 10) catch 0,
            .null => 0,
            .undefined => 0,
        };
    }

    pub fn asF64(self: JsValue) f64 {
        return switch (self) {
            .int => |v| @as(f64, @floatFromInt(v)),
            .float => |v| v,
            .bool => |v| if (v) 1.0 else 0.0,
            .string => |v| std.fmt.parseFloat(f64, v) catch 0,
            .null => 0.0,
            .undefined => std.math.nan(f64),
        };
    }

    pub fn asString(self: JsValue, alloc: Allocator) []const u8 {
        return switch (self) {
            .int => |v| std.fmt.allocPrint(alloc, "{}", .{v}) catch "",
            .float => |v| std.fmt.allocPrint(alloc, "{}", .{v}) catch "",
            .bool => |v| if (v) "true" else "false",
            .string => |v| v,
            .null => "null",
            .undefined => "undefined",
        };
    }

    pub fn asBool(self: JsValue) bool {
        return switch (self) {
            .int => |v| v != 0,
            .float => |v| v != 0.0,
            .bool => |v| v,
            .string => |v| v.len != 0,
            .null => false,
            .undefined => false,
        };
    }

    // --- comparison (JS == loose semantics, not ===) ---

    pub fn eq(self: JsValue, other: JsValue) bool {
        // Same type: direct comparison
        if (@as(std.meta.Tag(JsValue), self) == @as(std.meta.Tag(JsValue), other)) {
            return switch (self) {
                .int => |a| a == other.int,
                .float => |a| a == other.float,
                .bool => |a| a == other.bool,
                .string => |a| std.mem.eql(u8, a, other.string),
                .null => true,
                .undefined => true,
            };
        }
        // null == undefined → true (JS loose ==)
        if ((self == .null or self == .undefined) and (other == .null or other == .undefined))
            return true;
        // null/undefined are NOT loosely equal to any other type.
        // Per ECMA-262 §7.2.13: `null == 0` is false, `null == ""` is false,
        // `null == false` is false, `undefined == 0` is false, etc. The earlier
        // null==undefined branch is the only special case for these two tags.
        // Without this guard, the cross-type fallthrough below would coerce
        // `null.asF64()` (== 0.0) and `0.asF64()` (== 0.0) to equal, producing
        // the wrong `null == 0` → true result.
        if ((self == .null or self == .undefined) != (other == .null or other == .undefined))
            return false;
        // Cross-type: coerce both to f64 and compare (JS loose == semantics).
        // This handles: int vs float, int vs string, int vs bool,
        //               float vs string, float vs bool, string vs bool.
        return self.asF64() == other.asF64();
    }

    /// Strict equality (===): same type AND same value. No coercion.
    /// JS semantics: int and float are the same "number" type (3 === 3.0 is true).
    pub fn strictEq(self: JsValue, other: JsValue) bool {
        // JS numbers: int↔float should compare equal when values match
        if (self.isNumber() and other.isNumber()) {
            return self.asF64() == other.asF64();
        }
        if (@as(std.meta.Tag(JsValue), self) != @as(std.meta.Tag(JsValue), other))
            return false;
        return switch (self) {
            .int => |a| a == other.int,
            .float => |a| a == other.float,
            .bool => |a| a == other.bool,
            .string => |a| std.mem.eql(u8, a, other.string),
            .null => true,
            .undefined => true,
        };
    }
};

// ═══════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════

test "eq: null/undefined only loosely equal to null/undefined (R5-9)" {
    // ECMA-262 §7.2.13: `null == undefined` → true; `null`/`undefined` are
    // NOT loosely equal to any other type. Pre-fix the cross-type fallthrough
    // coerced via `asF64()`, so `null.asF64()` (= 0.0) == `0.asF64()` (= 0.0)
    // made `null == 0` (and `null == ""`, `null == false`) wrongly true.
    const Jv = JsValue;
    const undef: JsValue = .{ .undefined = {} };
    // The only true cases involving null/undefined.
    try std.testing.expect(Jv.fromNull().eq(Jv.fromNull()));
    try std.testing.expect(Jv.fromNull().eq(undef));
    try std.testing.expect(undef.eq(Jv.fromNull()));
    try std.testing.expect(undef.eq(undef));
    // null/undefined vs any other type must be false (pre-fix returned true
    // for cases that asF64-coerce to 0.0, e.g. null==0, null=="", null==false).
    try std.testing.expect(!Jv.fromNull().eq(Jv.fromI64(0)));
    try std.testing.expect(!Jv.fromNull().eq(Jv.fromF64(0.0)));
    try std.testing.expect(!Jv.fromNull().eq(Jv.fromBool(false)));
    try std.testing.expect(!Jv.fromNull().eq(Jv.fromString("")));
    try std.testing.expect(!undef.eq(Jv.fromI64(0)));
    try std.testing.expect(!undef.eq(Jv.fromF64(0.0)));
    try std.testing.expect(!undef.eq(Jv.fromBool(false)));
    try std.testing.expect(!undef.eq(Jv.fromString("")));
    // Sanity: cross-type numeric equality still works (e.g. 1 == 1.0).
    try std.testing.expect(Jv.fromI64(1).eq(Jv.fromF64(1.0)));
    // NaN != NaN (IEEE 754) — must not regress to true via the fallthrough.
    try std.testing.expect(!Jv.fromF64(std.math.nan(f64)).eq(Jv.fromF64(std.math.nan(f64))));
}

test "asI64: NaN/Inf/out-of-range coerce to 0 (R5-10)" {
    // Pre-fix: `asI64` used `@as(i64, @intFromFloat(v))` which panics in
    // Zig safe mode for NaN, ±Inf, or values outside i64 range. ECMA-262
    // ToInteger/ToInt32 yield 0 for these — the fix matches and also makes
    // `asI64` total (no panic) by guarding before the cast.
    const Jv = JsValue;
    try std.testing.expectEqual(@as(i64, 0), Jv.fromF64(std.math.nan(f64)).asI64());
    try std.testing.expectEqual(@as(i64, 0), Jv.fromF64(std.math.inf(f64)).asI64());
    try std.testing.expectEqual(@as(i64, 0), Jv.fromF64(-std.math.inf(f64)).asI64());
    // Out-of-i64-range f64 → 0 (avoids @intFromFloat panic in safe mode).
    try std.testing.expectEqual(@as(i64, 0), Jv.fromF64(1e30).asI64());
    try std.testing.expectEqual(@as(i64, 0), Jv.fromF64(-1e30).asI64());
    // Normal float values still convert correctly (no regression).
    try std.testing.expectEqual(@as(i64, 42), Jv.fromF64(42.0).asI64());
    try std.testing.expectEqual(@as(i64, -7), Jv.fromF64(-7.0).asI64());
    // Integer/bool-typed values are unaffected by the float guards.
    try std.testing.expectEqual(@as(i64, 100), Jv.fromI64(100).asI64());
    try std.testing.expectEqual(@as(i64, 1), Jv.fromBool(true).asI64());
    try std.testing.expectEqual(@as(i64, 0), Jv.fromBool(false).asI64());
}
