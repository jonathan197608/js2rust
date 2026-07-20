const std = @import("std");

/// Arbitrary-precision integer, wrapping `std.math.big.int.Managed`.
/// Lifetime is tied to the allocator passed at init.
pub const JsBigInt = struct {
    value: std.math.big.int.Managed,

    const Self = @This();

    /// Initialize from a decimal string (no trailing `n`).
    /// Also supports hex (`0x`), octal (`0o`), and binary (`0b`) prefixes.
    /// Caller owns the returned JsBigInt; call `deinit()` when done.
    pub fn init(alloc: std.mem.Allocator, s: []const u8) !Self {
        var managed = try std.math.big.int.Managed.init(alloc);
        errdefer managed.deinit();
        // Detect base from prefix: 0x → 16, 0o → 8, 0b → 2, else 10
        const base: u8 = if (s.len > 2 and s[0] == '0') switch (s[1]) {
            'x', 'X' => 16,
            'o', 'O' => 8,
            'b', 'B' => 2,
            else => 10,
        } else 10;
        const digits: []const u8 = if (base != 10) s[2..] else s;
        try managed.setString(base, digits);
        return Self{ .value = managed };
    }

    /// Create from a native i64 (for `BigInt(n)` constructor).
    pub fn fromI64(alloc: std.mem.Allocator, v: i64) !Self {
        var managed = try std.math.big.int.Managed.init(alloc);
        errdefer managed.deinit();
        try managed.set(v);
        return Self{ .value = managed };
    }

    /// Create from either a string or i64 value (for `BigInt(x)` constructor).
    /// Uses comptime type detection to dispatch to `init` or `fromI64`.
    pub fn fromValue(alloc: std.mem.Allocator, v: anytype) !Self {
        const T = @TypeOf(v);
        switch (@typeInfo(T)) {
            .int => return fromI64(alloc, v),
            .comptime_int => return fromI64(alloc, @as(i64, v)),
            .pointer => |ptr| {
                // []const u8 slice
                if (ptr.size == .slice and ptr.child == u8) {
                    return init(alloc, v);
                }
                // *const [N:0]u8 or *const [N]u8 string literal
                if (ptr.size == .one) {
                    const Child = ptr.child;
                    if (@typeInfo(Child) == .array) {
                        const arr = @typeInfo(Child).array;
                        if (arr.child == u8) {
                            return init(alloc, v[0..arr.len]);
                        }
                    }
                }
                @compileError("BigInt.fromValue: unsupported pointer type " ++ @typeName(T));
            },
            else => {
                @compileError("BigInt.fromValue: unsupported type " ++ @typeName(T));
            },
        }
    }

    pub fn deinit(self: *Self, _: std.mem.Allocator) void {
        self.value.deinit();
    }

    // ---- arithmetic ----

    pub fn add(self: *const Self, other: *const Self, alloc: std.mem.Allocator) !Self {
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        try result.add(&self.value, &other.value);
        return Self{ .value = result };
    }

    pub fn sub(self: *const Self, other: *const Self, alloc: std.mem.Allocator) !Self {
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        try result.sub(&self.value, &other.value);
        return Self{ .value = result };
    }

    pub fn mul(self: *const Self, other: *const Self, alloc: std.mem.Allocator) !Self {
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        try result.mul(&self.value, &other.value);
        return Self{ .value = result };
    }

    pub fn div(self: *const Self, other: *const Self, alloc: std.mem.Allocator) !Self {
        // BigInt division by zero is an error (matches JS RangeError behavior)
        if (other.value.eqlZero()) return error.DivisionByZero;
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        var remainder = try std.math.big.int.Managed.init(alloc);
        defer remainder.deinit();
        // BigInt division truncates toward zero (like JS)
        try result.divTrunc(&remainder, &self.value, &other.value);
        return Self{ .value = result };
    }

    /// Exponentiation: self ^ exp.
    /// `exp` is cast to u32 (Zig 0.16.0 Managed.pow requires u32 exponent).
    /// Returns `error.RangeError` if `exp > maxInt(u32)` — guarding prevents
    /// a runtime panic from `@intCast` (R8 P0-4). Negative exponents never
    /// reach here: callers route BigInt power operands through `toU64()`,
    /// which errors on negative values and is converted upstream to
    /// `error.JsThrow` (see `emit_bigint_binary` in `emit/expr/binary.rs`).
    pub fn pow(self: *const Self, exp: u64, alloc: std.mem.Allocator) !Self {
        if (exp > std.math.maxInt(u32)) return error.RangeError;
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        // Managed.pow(r, a, b): r = a ^ b (b is now u32 in Zig 0.16.0)
        try std.math.big.int.Managed.pow(&result, &self.value, @intCast(exp));
        return Self{ .value = result };
    }

    /// Remainder (modulo): self % other.
    /// JS BigInt remainder matches truncated division (sign of dividend).
    pub fn rem(self: *const Self, other: *const Self, alloc: std.mem.Allocator) !Self {
        if (other.value.eqlZero()) return error.DivisionByZero;
        var quotient = try std.math.big.int.Managed.init(alloc);
        defer quotient.deinit();
        var remainder = try std.math.big.int.Managed.init(alloc);
        errdefer remainder.deinit();
        try quotient.divTrunc(&remainder, &self.value, &other.value);
        return Self{ .value = remainder };
    }

    // ---- bitwise ----

    pub fn bitwiseAnd(self: *const Self, other: *const Self, alloc: std.mem.Allocator) !Self {
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        try result.bitAnd(&self.value, &other.value);
        return Self{ .value = result };
    }

    pub fn bitwiseOr(self: *const Self, other: *const Self, alloc: std.mem.Allocator) !Self {
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        try result.bitOr(&self.value, &other.value);
        return Self{ .value = result };
    }

    pub fn bitwiseXor(self: *const Self, other: *const Self, alloc: std.mem.Allocator) !Self {
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        try result.bitXor(&self.value, &other.value);
        return Self{ .value = result };
    }

    /// BigInt left shift. In JS, negative shift amounts reverse direction:
    /// `x << -n` is equivalent to `x >> n`.
    pub fn shiftLeft(self: *const Self, shift: i64, alloc: std.mem.Allocator) !Self {
        if (shift < 0) {
            // Use @abs which returns u64 to avoid -minInt(i64) overflow
            const abs_shift: usize = @intCast(@abs(shift));
            return self.shiftRightRaw(abs_shift, alloc);
        }
        return self.shiftLeftRaw(@intCast(shift), alloc);
    }

    /// BigInt right shift. In JS, negative shift amounts reverse direction:
    /// `x >> -n` is equivalent to `x << n`.
    pub fn shiftRight(self: *const Self, shift: i64, alloc: std.mem.Allocator) !Self {
        if (shift < 0) {
            const abs_shift: usize = @intCast(@abs(shift));
            return self.shiftLeftRaw(abs_shift, alloc);
        }
        return self.shiftRightRaw(@intCast(shift), alloc);
    }

    fn shiftLeftRaw(self: *const Self, shift: usize, alloc: std.mem.Allocator) !Self {
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        try result.shiftLeft(&self.value, shift);
        return Self{ .value = result };
    }

    fn shiftRightRaw(self: *const Self, shift: usize, alloc: std.mem.Allocator) !Self {
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        try result.shiftRight(&self.value, shift);
        return Self{ .value = result };
    }

    pub fn neg(self: *const Self, alloc: std.mem.Allocator) !Self {
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        try result.copy(self.value.toConst());
        result.negate();
        return Self{ .value = result };
    }

    /// Bitwise NOT (~x = -(x + 1) in two's complement)
    pub fn bitwiseNot(self: *const Self, alloc: std.mem.Allocator) !Self {
        var one = try std.math.big.int.Managed.init(alloc);
        defer one.deinit();
        try one.set(1);
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        try result.add(&self.value, &one);
        result.negate();
        return Self{ .value = result };
    }

    // ---- comparison ----

    pub fn eq(self: *const Self, other: *const Self) bool {
        return self.value.order(other.value) == .eq;
    }

    pub fn isZero(self: *const Self) bool {
        return self.value.toConst().eqlZero();
    }

    pub fn order(self: *const Self, other: *const Self) std.math.Order {
        return self.value.order(other.value);
    }

    // ---- conversion ----

    pub fn toI64(self: *const Self) !i64 {
        return try self.value.toConst().toInt(i64);
    }

    pub fn toU64(self: *const Self) !u64 {
        return try self.value.toConst().toInt(u64);
    }

    /// BigInt.prototype.toString([radix]) — convert to a string in the given base.
    /// R8-P1-4: Previously this method hard-coded base 10 and accepted no radix
    /// parameter, so JS callers' radix arguments were silently dropped.
    /// Now validates radix in [2, 36] per ECMA-262 21.1.3.0 and defers to
    /// std.math.big.int.Managed.toString which natively supports base 2..36
    /// with lowercase digits and a leading `-` for negative values.
    /// Examples: 255n.toString(16) -> "ff", (-255n).toString(16) -> "-ff".
    pub fn toString(self: *const Self, alloc: std.mem.Allocator, radix: i64) ![]u8 {
        if (radix < 2 or radix > 36) return error.RangeError;
        return try self.value.toString(alloc, @as(u8, @intCast(radix)), .lower);
    }

    /// BigInt.prototype.valueOf() — returns self (identity).
    /// In JS, `bigint.valueOf()` returns the BigInt primitive value itself.
    pub fn valueOf(self: *const Self) *const Self {
        return self;
    }

    /// Format the BigInt value for display (implements Zig 0.16.0 std.fmt interface).
    /// Outputs the decimal representation with trailing `n` suffix,
    /// matching Node.js console.log output (e.g. `5n`).
    /// Invoked via the `{f}` format specifier.
    pub fn format(self: *const Self, w: *std.Io.Writer) std.Io.Writer.Error!void {
        try self.value.format(w);
        try w.writeAll("n");
    }
};

/// BigInt.asIntN(width, bigint) — clamp to signed N-bit integer.
/// Wraps the BigInt value to a signed integer of `width` bits using two's complement.
pub fn asIntN(bits: u64, value: *const JsBigInt, alloc: std.mem.Allocator) !JsBigInt {
    if (bits == 0) {
        var zero = try std.math.big.int.Managed.init(alloc);
        errdefer zero.deinit();
        try zero.set(0);
        return JsBigInt{ .value = zero };
    }
    // Compute 2^(bits-1) as the min/max bound for signed representation
    var modulus = try std.math.big.int.Managed.init(alloc);
    defer modulus.deinit();
    var one = try std.math.big.int.Managed.init(alloc);
    defer one.deinit();
    try one.set(1);
    try modulus.set(1);
    try modulus.shiftLeft(&modulus, @intCast(bits - 1));
    // modulus = 2^(bits-1)
    // For asIntN: result = value mod 2^bits, then if >= 2^(bits-1), subtract 2^bits
    var full_mod = try std.math.big.int.Managed.init(alloc);
    defer full_mod.deinit();
    try full_mod.set(1);
    try full_mod.shiftLeft(&full_mod, @intCast(bits));
    // full_mod = 2^bits

    var remainder = try std.math.big.int.Managed.init(alloc);
    errdefer remainder.deinit();
    var quotient = try std.math.big.int.Managed.init(alloc);
    defer quotient.deinit();
    try quotient.divTrunc(&remainder, &value.value, &full_mod);

    // R8-P1-5: divTrunc truncates toward zero, so the remainder takes the
    // sign of the dividend — NOT "always non-negative" as the old comment
    // claimed. For negative inputs this produced wrong results:
    //   asIntN(8, -129n) returned -129 instead of 127.
    // Normalize to the [0, 2^bits) range first, then apply signed conversion.
    // Note: Managed.isPositive() returns false for negative values and true for
    // zero/positive, so !isPositive() implies negative-and-nonzero.
    if (!remainder.isPositive() and !remainder.eqlZero()) {
        try remainder.add(&remainder, &full_mod);
    }

    // If remainder >= 2^(bits-1), subtract 2^bits to get signed value
    const order = remainder.order(modulus);
    if (order != .lt) {
        try remainder.sub(&remainder, &full_mod);
    }
    return JsBigInt{ .value = remainder };
}

/// BigInt.asUintN(width, bigint) — clamp to unsigned N-bit integer.
/// Wraps the BigInt value to an unsigned integer of `width` bits.
pub fn asUintN(bits: u64, value: *const JsBigInt, alloc: std.mem.Allocator) !JsBigInt {
    if (bits == 0) {
        var zero = try std.math.big.int.Managed.init(alloc);
        errdefer zero.deinit();
        try zero.set(0);
        return JsBigInt{ .value = zero };
    }
    // Compute 2^bits as the modulus
    var modulus = try std.math.big.int.Managed.init(alloc);
    defer modulus.deinit();
    try modulus.set(1);
    try modulus.shiftLeft(&modulus, @intCast(bits));

    var remainder = try std.math.big.int.Managed.init(alloc);
    errdefer remainder.deinit();
    var quotient = try std.math.big.int.Managed.init(alloc);
    defer quotient.deinit();
    try quotient.divTrunc(&remainder, &value.value, &modulus);

    // R8-P1-5: divTrunc remainder takes the sign of the dividend, NOT
    // "always non-negative" as the old comment claimed. For negative inputs
    // this produced wrong results:
    //   asUintN(8, -1n) returned -1n instead of 255n.
    // Normalize to the [0, 2^bits) range.
    // Note: Managed.isPositive() returns false for negative values and true for
    // zero/positive, so !isPositive() implies negative-and-nonzero.
    if (!remainder.isPositive() and !remainder.eqlZero()) {
        try remainder.add(&remainder, &modulus);
    }
    return JsBigInt{ .value = remainder };
}

// ── Tests ────────────────────────────────────────────────────────

test "shiftLeft/shiftRight with minInt(i64) shift does not panic (R7-7)" {
    // Pre-fix: negating minInt(i64) in `-shift` overflowed i64.
    // Using @abs which returns u64 avoids the overflow.
    const alloc = std.testing.allocator;
    var n = try JsBigInt.fromI64(alloc, 1);
    defer n.deinit(alloc);

    // x << minInt(i64) → effectively x >> |minInt| (a huge right shift → 0)
    // This would allocate enormous memory with a huge left shift, so test
    // the reverse direction (shiftLeft with minInt = right shift → 0).
    var result = try n.shiftLeft(std.math.minInt(i64), alloc);
    defer result.deinit(alloc);
    try std.testing.expect(result.isZero());

    // Also test a moderate negative shift that won't OOM:
    // 1n >> -2 = 1n << 2 = 4n
    var result2 = try n.shiftRight(-2, alloc);
    defer result2.deinit(alloc);
    var expected = try JsBigInt.fromI64(alloc, 4);
    defer expected.deinit(alloc);
    try std.testing.expect(result2.eq(&expected));

    // 8n << -2 = 8n >> 2 = 2n
    var eight = try JsBigInt.fromI64(alloc, 8);
    defer eight.deinit(alloc);
    var result3 = try eight.shiftLeft(-2, alloc);
    defer result3.deinit(alloc);
    var expected2 = try JsBigInt.fromI64(alloc, 2);
    defer expected2.deinit(alloc);
    try std.testing.expect(result3.eq(&expected2));
}

test "pow with exp > maxInt(u32) returns RangeError not panic (R8 P0-4)" {
    const alloc = std.testing.allocator;
    var base = try JsBigInt.fromI64(alloc, 2);
    defer base.deinit(alloc);

    // Pre-fix: @intCast(exp) from u64 to u32 would panic for exp > 2^32.
    // Post-fix: returns error.RangeError which the emitter routes to error.JsThrow.
    const huge_exp: u64 = (@as(u64, std.math.maxInt(u32)) + 1);
    const result = base.pow(huge_exp, alloc);
    try std.testing.expectError(error.RangeError, result);

    // Normal case still works: 2^10 = 1024.
    var result_ok = try base.pow(10, alloc);
    defer result_ok.deinit(alloc);
    var expected = try JsBigInt.fromI64(alloc, 1024);
    defer expected.deinit(alloc);
    try std.testing.expect(result_ok.eq(&expected));
}

test "asIntN wraps negative values correctly (R8-P1-5)" {
    const alloc = std.testing.allocator;

    // Pre-fix: divTrunc remainder took the sign of the dividend, so
    // asIntN(8, -129n) returned -129n instead of 127n.
    {
        var input = try JsBigInt.fromI64(alloc, -129);
        defer input.deinit(alloc);
        var result = try asIntN(8, &input, alloc);
        defer result.deinit(alloc);
        var expected = try JsBigInt.fromI64(alloc, 127);
        defer expected.deinit(alloc);
        try std.testing.expect(result.eq(&expected));
    }

    // asIntN(8, -1n) = -1n (unchanged, within signed range)
    {
        var input = try JsBigInt.fromI64(alloc, -1);
        defer input.deinit(alloc);
        var result = try asIntN(8, &input, alloc);
        defer result.deinit(alloc);
        var expected = try JsBigInt.fromI64(alloc, -1);
        defer expected.deinit(alloc);
        try std.testing.expect(result.eq(&expected));
    }

    // asIntN(8, 128n) = -128n (positive boundary wraps to min signed)
    {
        var input = try JsBigInt.fromI64(alloc, 128);
        defer input.deinit(alloc);
        var result = try asIntN(8, &input, alloc);
        defer result.deinit(alloc);
        var expected = try JsBigInt.fromI64(alloc, -128);
        defer expected.deinit(alloc);
        try std.testing.expect(result.eq(&expected));
    }

    // asIntN(8, 127n) = 127n (positive within range, unchanged)
    {
        var input = try JsBigInt.fromI64(alloc, 127);
        defer input.deinit(alloc);
        var result = try asIntN(8, &input, alloc);
        defer result.deinit(alloc);
        var expected = try JsBigInt.fromI64(alloc, 127);
        defer expected.deinit(alloc);
        try std.testing.expect(result.eq(&expected));
    }

    // asIntN(8, 255n) = -1n (unsigned max wraps to signed -1)
    {
        var input = try JsBigInt.fromI64(alloc, 255);
        defer input.deinit(alloc);
        var result = try asIntN(8, &input, alloc);
        defer result.deinit(alloc);
        var expected = try JsBigInt.fromI64(alloc, -1);
        defer expected.deinit(alloc);
        try std.testing.expect(result.eq(&expected));
    }

    // asIntN(0, any) = 0n
    {
        var input = try JsBigInt.fromI64(alloc, 42);
        defer input.deinit(alloc);
        var result = try asIntN(0, &input, alloc);
        defer result.deinit(alloc);
        try std.testing.expect(result.isZero());
    }
}

test "asUintN wraps negative values correctly (R8-P1-5)" {
    const alloc = std.testing.allocator;

    // Pre-fix: asUintN(8, -1n) returned -1n instead of 255n.
    {
        var input = try JsBigInt.fromI64(alloc, -1);
        defer input.deinit(alloc);
        var result = try asUintN(8, &input, alloc);
        defer result.deinit(alloc);
        var expected = try JsBigInt.fromI64(alloc, 255);
        defer expected.deinit(alloc);
        try std.testing.expect(result.eq(&expected));
    }

    // asUintN(8, -129n) = 127n
    {
        var input = try JsBigInt.fromI64(alloc, -129);
        defer input.deinit(alloc);
        var result = try asUintN(8, &input, alloc);
        defer result.deinit(alloc);
        var expected = try JsBigInt.fromI64(alloc, 127);
        defer expected.deinit(alloc);
        try std.testing.expect(result.eq(&expected));
    }

    // asUintN(8, 200n) = 200n (positive within range, unchanged)
    {
        var input = try JsBigInt.fromI64(alloc, 200);
        defer input.deinit(alloc);
        var result = try asUintN(8, &input, alloc);
        defer result.deinit(alloc);
        var expected = try JsBigInt.fromI64(alloc, 200);
        defer expected.deinit(alloc);
        try std.testing.expect(result.eq(&expected));
    }

    // asUintN(8, 256n) = 0n (wraps around)
    {
        var input = try JsBigInt.fromI64(alloc, 256);
        defer input.deinit(alloc);
        var result = try asUintN(8, &input, alloc);
        defer result.deinit(alloc);
        try std.testing.expect(result.isZero());
    }

    // asUintN(0, any) = 0n
    {
        var input = try JsBigInt.fromI64(alloc, 42);
        defer input.deinit(alloc);
        var result = try asUintN(0, &input, alloc);
        defer result.deinit(alloc);
        try std.testing.expect(result.isZero());
    }
}

test "BigInt.toString with radix (R8-P1-4)" {
    const alloc = std.testing.allocator;

    // Default decimal: 255n.toString(10) -> "255"
    {
        var b = try JsBigInt.fromI64(alloc, 255);
        defer b.deinit(alloc);
        const s = try b.toString(alloc, 10);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("255", s);
    }

    // Hex: 255n.toString(16) -> "ff"
    {
        var b = try JsBigInt.fromI64(alloc, 255);
        defer b.deinit(alloc);
        const s = try b.toString(alloc, 16);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("ff", s);
    }

    // Binary: 10n.toString(2) -> "1010"
    {
        var b = try JsBigInt.fromI64(alloc, 10);
        defer b.deinit(alloc);
        const s = try b.toString(alloc, 2);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("1010", s);
    }

    // Octal: 63n.toString(8) -> "77"
    {
        var b = try JsBigInt.fromI64(alloc, 63);
        defer b.deinit(alloc);
        const s = try b.toString(alloc, 8);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("77", s);
    }

    // Base 36: 35n.toString(36) -> "z"
    {
        var b = try JsBigInt.fromI64(alloc, 35);
        defer b.deinit(alloc);
        const s = try b.toString(alloc, 36);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("z", s);
    }

    // Large value: 65535n.toString(16) -> "ffff"
    {
        var b = try JsBigInt.fromI64(alloc, 65535);
        defer b.deinit(alloc);
        const s = try b.toString(alloc, 16);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("ffff", s);
    }

    // Zero: 0n.toString(16) -> "0"
    {
        var b = try JsBigInt.fromI64(alloc, 0);
        defer b.deinit(alloc);
        const s = try b.toString(alloc, 16);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("0", s);
    }

    // Negative hex: (-255n).toString(16) -> "-ff"
    {
        var b = try JsBigInt.fromI64(alloc, -255);
        defer b.deinit(alloc);
        const s = try b.toString(alloc, 16);
        defer alloc.free(s);
        try std.testing.expectEqualStrings("-ff", s);
    }

    // Out-of-range radix throws RangeError
    {
        var b = try JsBigInt.fromI64(alloc, 42);
        defer b.deinit(alloc);
        try std.testing.expectError(error.RangeError, b.toString(alloc, 1));
        try std.testing.expectError(error.RangeError, b.toString(alloc, 37));
    }
}
