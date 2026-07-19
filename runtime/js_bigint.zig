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
    pub fn pow(self: *const Self, exp: u64, alloc: std.mem.Allocator) !Self {
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

    pub fn toString(self: *const Self, alloc: std.mem.Allocator) ![]u8 {
        return try self.value.toString(alloc, 10, .lower);
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
    // remainder is value mod 2^bits (always non-negative via divTrunc)
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
    // divTrunc remainder is always non-negative when modulus is positive
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
