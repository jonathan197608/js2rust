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

    /// Custom formatter so `std.fmt.allocPrint("{f}", .{jsvalue})` outputs the JS string representation.
    /// Note: in Zig 0.16.0, only the `{f}` specifier dispatches to this method;
    /// `{}`/`{any}` emit the debug representation for tagged unions.
    pub fn format(self: JsValue, writer: *std.Io.Writer) std.Io.Writer.Error!void {
        switch (self) {
            .int => |v| try writer.print("{d}", .{v}),
            // R8-P1-7: Zig {d} emits lowercase "nan"/"inf"; JS requires
            // "NaN"/"Infinity"/"-Infinity".
            .float => |v| {
                if (std.math.isNan(v)) {
                    try writer.writeAll("NaN");
                } else if (std.math.isInf(v)) {
                    try writer.writeAll(if (v > 0) "Infinity" else "-Infinity");
                } else {
                    try writer.print("{d}", .{v});
                }
            },
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
            .string => |v| blk: {
                // JS ToInteger/ToInt32 trims whitespace before parsing.
                // Pre-fix: std.fmt.parseInt on "  42" fails → returns 0.
                var start: usize = 0;
                while (start < v.len and std.ascii.isWhitespace(v[start])) start += 1;
                const trimmed = v[start..];
                // R18-RT-1: parseInt("3.14") fails → returns 0, but JS ToNumber("3.14")|0 = 3.
                // Try integer parse first, then fall back to float parse + @trunc.
                if (std.fmt.parseInt(i64, trimmed, 10)) |n| {
                    break :blk n;
                } else |_| {
                    const f = stringToNumber(trimmed) catch break :blk 0;
                    if (std.math.isNan(f) or std.math.isInf(f)) break :blk 0;
                    const max_i64_f: f64 = 9.223372036854776e18;
                    const min_i64_f: f64 = -9.223372036854776e18;
                    if (f > max_i64_f or f < min_i64_f) break :blk 0;
                    break :blk @as(i64, @intFromFloat(@trunc(f)));
                }
            },
            .null => 0,
            .undefined => 0,
        };
    }

    pub fn asF64(self: JsValue) f64 {
        return switch (self) {
            .int => |v| @as(f64, @floatFromInt(v)),
            .float => |v| v,
            .bool => |v| if (v) 1.0 else 0.0,
            // R8-P1-6: JS ToNumber("") == +0 and ToNumber("   ") == +0
            // (parseFloat fails for these; ECMA-262 StringNumericLiteral
            // returns +0 after trimming WhiteSpace+LineTerminator).
            // Non-numeric ("abc") still yields NaN per R7-5.
            // R16: Trim leading whitespace before parsing (JS ToNumber trims).
            // R18: Use shared stringToNumber helper that handles 0x/0o/0b prefixes.
            .string => |v| blk: {
                var start: usize = 0;
                while (start < v.len and std.ascii.isWhitespace(v[start])) start += 1;
                const trimmed = v[start..];
                if (stringToNumber(trimmed)) |f| {
                    break :blk f;
                } else |_| {
                    // Check if it's all whitespace → 0, otherwise NaN
                    var all_ws = true;
                    for (trimmed) |byte| {
                        switch (byte) {
                            ' ', '\t', '\n', '\r', 0x0B, 0x0C => {},
                            else => {
                                all_ws = false;
                                break;
                            },
                        }
                    }
                    break :blk if (all_ws) 0.0 else std.math.nan(f64);
                }
            },
            .null => 0.0,
            .undefined => std.math.nan(f64),
        };
    }

    pub fn asString(self: JsValue, alloc: Allocator) []const u8 {
        return switch (self) {
            .int => |v| std.fmt.allocPrint(alloc, "{}", .{v}) catch "",
            // R8-P1-7: Zig emits lowercase "nan"/"inf"; JS requires
            // "NaN"/"Infinity"/"-Infinity".
            .float => |v| blk: {
                if (std.math.isNan(v)) break :blk "NaN";
                if (std.math.isInf(v)) break :blk if (v > 0) "Infinity" else "-Infinity";
                break :blk std.fmt.allocPrint(alloc, "{}", .{v}) catch "";
            },
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

    /// SameValue (ECMA-262 §7.2.10): used by Object.is.
    /// Differs from === in two cases:
    ///   - NaN sameValue NaN → true  (=== returns false)
    ///   - +0 sameValue -0  → false  (=== returns true)
    pub fn sameValue(self: JsValue, other: JsValue) bool {
        // Different tags (after number unification) → false
        const self_is_num = self.isNumber();
        const other_is_num = other.isNumber();
        if (self_is_num and other_is_num) {
            const a = self.asF64();
            const b = other.asF64();
            // NaN === NaN → true (unlike ===)
            if (std.math.isNan(a) and std.math.isNan(b)) return true;
            // +0 vs -0 → false (unlike ===)
            if (a == 0.0 and b == 0.0) {
                return std.math.signbit(a) == std.math.signbit(b);
            }
            return a == b;
        }
        if (@as(std.meta.Tag(JsValue), self) != @as(std.meta.Tag(JsValue), other))
            return false;
        return switch (self) {
            .int => |a| a == other.int,
            .float => unreachable, // handled above
            .bool => |a| a == other.bool,
            .string => |a| std.mem.eql(u8, a, other.string),
            .null => true,
            .undefined => true,
        };
    }
};

/// R18: Shared string-to-number conversion per ECMA-262 §7.1.4.1.
/// Handles hex ("0x"), octal ("0o"), binary ("0b") prefixes and float literals.
/// `trimmed` must already have leading whitespace removed.
fn stringToNumber(trimmed: []const u8) error{ParseError}!f64 {
    if (trimmed.len == 0) return 0.0;
    // Handle 0x/0X hex prefix
    if (trimmed.len > 2 and trimmed[0] == '0' and (trimmed[1] == 'x' or trimmed[1] == 'X')) {
        const n = std.fmt.parseInt(i64, trimmed[2..], 16) catch return error.ParseError;
        return @as(f64, @floatFromInt(n));
    }
    // R18-RT-2: Handle 0o/0O octal prefix (ECMA-262 §7.1.4.1)
    if (trimmed.len > 2 and trimmed[0] == '0' and (trimmed[1] == 'o' or trimmed[1] == 'O')) {
        const n = std.fmt.parseInt(i64, trimmed[2..], 8) catch return error.ParseError;
        return @as(f64, @floatFromInt(n));
    }
    // R18-RT-2: Handle 0b/0B binary prefix (ECMA-262 §7.1.4.1)
    if (trimmed.len > 2 and trimmed[0] == '0' and (trimmed[1] == 'b' or trimmed[1] == 'B')) {
        const n = std.fmt.parseInt(i64, trimmed[2..], 2) catch return error.ParseError;
        return @as(f64, @floatFromInt(n));
    }
    return std.fmt.parseFloat(f64, trimmed) catch error.ParseError;
}

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

test "asF64: non-numeric string → NaN not 0 (R7-5)" {
    // Pre-fix: `asF64` for `.string` used `parseFloat(...) catch 0`, so
    // `Number("hello")` returned 0 instead of NaN per ECMA-262 ToNumber.
    const Jv = JsValue;
    try std.testing.expect(std.math.isNan(Jv.fromString("hello").asF64()));
    try std.testing.expect(std.math.isNan(Jv.fromString("abc").asF64()));
    // Numeric strings still parse correctly (no regression).
    try std.testing.expectEqual(@as(f64, 42.0), Jv.fromString("42").asF64());
    try std.testing.expectEqual(@as(f64, 3.14), Jv.fromString("3.14").asF64());
}

test "asF64: empty/whitespace string → 0 not NaN (R8-P1-6)" {
    // JS Number("")===0, Number(" ")===0, Number("  \t\n ")===0.
    // Pre-fix: parseFloat("") fails → NaN, violating ECMA-262 ToNumber.
    const Jv = JsValue;
    try std.testing.expectEqual(@as(f64, 0.0), Jv.fromString("").asF64());
    try std.testing.expectEqual(@as(f64, 0.0), Jv.fromString(" ").asF64());
    try std.testing.expectEqual(@as(f64, 0.0), Jv.fromString("\t\n\r ").asF64());
    // Non-numeric, non-whitespace still → NaN (regression guard).
    try std.testing.expect(std.math.isNan(Jv.fromString("abc").asF64()));
    try std.testing.expect(std.math.isNan(Jv.fromString("12abc").asF64()));
}

test "asI64: whitespace-prefixed string trims before parsing (R16)" {
    // JS: "  42" | 0 → 42 (ToInt32 trims whitespace).
    // Pre-fix: std.fmt.parseInt("  42") fails → returned 0.
    const Jv = JsValue;
    try std.testing.expectEqual(@as(i64, 42), Jv.fromString("  42").asI64());
    try std.testing.expectEqual(@as(i64, -7), Jv.fromString("\t-7").asI64());
    try std.testing.expectEqual(@as(i64, 0), Jv.fromString("  abc").asI64());
}

test "asF64: whitespace-prefixed string trims before parsing (R16)" {
    // JS: Number("  42") === 42, Number("\t3.14") === 3.14.
    // Pre-fix: parseFloat("  42") fails → NaN.
    const Jv = JsValue;
    try std.testing.expectEqual(@as(f64, 42.0), Jv.fromString("  42").asF64());
    try std.testing.expectEqual(@as(f64, 3.14), Jv.fromString("\t3.14").asF64());
}

test "asF64: 0x hex string parses to integer (R16)" {
    // JS: Number("0xFF") === 255, Number("0x1A") === 26.
    // Pre-fix: parseFloat("0xFF") fails → NaN.
    const Jv = JsValue;
    try std.testing.expectEqual(@as(f64, 255.0), Jv.fromString("0xFF").asF64());
    try std.testing.expectEqual(@as(f64, 26.0), Jv.fromString("0x1A").asF64());
    try std.testing.expect(std.math.isNan(Jv.fromString("0x").asF64()));
}

test "asI64: float string truncates to integer (R18-RT-1)" {
    // JS: "3.14" | 0 → 3, "1e5" | 0 → 100000, "0xFF" | 0 → 255.
    // Pre-fix: parseInt("3.14") fails → 0.
    const Jv = JsValue;
    try std.testing.expectEqual(@as(i64, 3), Jv.fromString("3.14").asI64());
    try std.testing.expectEqual(@as(i64, 100000), Jv.fromString("1e5").asI64());
    try std.testing.expectEqual(@as(i64, 255), Jv.fromString("0xFF").asI64());
    try std.testing.expectEqual(@as(i64, -3), Jv.fromString("-3.9").asI64());
    try std.testing.expectEqual(@as(i64, 0), Jv.fromString("0xZZ").asI64());
}

test "asI64: huge float string does not panic (R20-RT-1)" {
    // JS: "99999999999999999999" | 0 → 0 (out of i64 range).
    // Pre-fix: @intFromFloat(@trunc(f)) panicked on out-of-range f64.
    const Jv = JsValue;
    try std.testing.expectEqual(@as(i64, 0), Jv.fromString("99999999999999999999").asI64());
    try std.testing.expectEqual(@as(i64, 0), Jv.fromString("-99999999999999999999").asI64());
    try std.testing.expectEqual(@as(i64, 0), Jv.fromString("1e30").asI64());
    try std.testing.expectEqual(@as(i64, 42), Jv.fromString("42.9").asI64());
}

test "asF64: 0o octal and 0b binary string parses (R18-RT-2)" {
    // JS: Number("0o17") === 15, Number("0b101") === 5.
    // Pre-fix: parseFloat("0o17") fails → NaN.
    const Jv = JsValue;
    try std.testing.expectEqual(@as(f64, 15.0), Jv.fromString("0o17").asF64());
    try std.testing.expectEqual(@as(f64, 15.0), Jv.fromString("0O17").asF64());
    try std.testing.expectEqual(@as(f64, 5.0), Jv.fromString("0b101").asF64());
    try std.testing.expectEqual(@as(f64, 5.0), Jv.fromString("0B101").asF64());
    try std.testing.expect(std.math.isNan(Jv.fromString("0o").asF64()));
}

test "format/asString emit NaN/Infinity not nan/inf (R8-P1-7)" {
    const a = std.testing.allocator;
    const Jv = JsValue;

    // format() via allocPrint("{f}", .{v}) — {f} is the specifier that
    // dispatches to the custom format method (Zig 0.16.0 Writer.zig).
    // Pre-fix: the method body used {d} which emits lowercase "nan"/"inf".
    {
        const s = try std.fmt.allocPrint(a, "{f}", .{Jv.fromF64(std.math.nan(f64))});
        defer a.free(s);
        try std.testing.expectEqualStrings("NaN", s);
    }
    {
        const s = try std.fmt.allocPrint(a, "{f}", .{Jv.fromF64(std.math.inf(f64))});
        defer a.free(s);
        try std.testing.expectEqualStrings("Infinity", s);
    }
    {
        const s = try std.fmt.allocPrint(a, "{f}", .{Jv.fromF64(-std.math.inf(f64))});
        defer a.free(s);
        try std.testing.expectEqualStrings("-Infinity", s);
    }

    // asString() — NaN/Inf return static string literals (no alloc, no free).
    try std.testing.expectEqualStrings("NaN", Jv.fromF64(std.math.nan(f64)).asString(a));
    try std.testing.expectEqualStrings("Infinity", Jv.fromF64(std.math.inf(f64)).asString(a));
    try std.testing.expectEqualStrings("-Infinity", Jv.fromF64(-std.math.inf(f64)).asString(a));

    // Normal float still formats via {d} (no regression).
    {
        const s = try std.fmt.allocPrint(a, "{f}", .{Jv.fromF64(3.5)});
        defer a.free(s);
        try std.testing.expectEqualStrings("3.5", s);
    }
}
