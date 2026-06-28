const std = @import("std");

/// Arbitrary-precision integer, wrapping `std.math.big.int.Managed`.
/// Lifetime is tied to the allocator passed at init.
pub const JsBigInt = struct {
    value: std.math.big.int.Managed,

    const Self = @This();

    /// Initialize from a decimal string (no trailing `n`).
    /// Caller owns the returned JsBigInt; call `deinit()` when done.
    pub fn init(alloc: std.mem.Allocator, s: []const u8) !Self {
        var managed = try std.math.big.int.Managed.init(alloc);
        errdefer managed.deinit();
        // `Managed.setString` supports base 10
        try managed.setString(10, s);
        return Self{ .value = managed };
    }

    /// Create from a native i64 (for `BigInt(n)` constructor).
    pub fn fromI64(alloc: std.mem.Allocator, v: i64) !Self {
        var managed = try std.math.big.int.Managed.init(alloc);
        errdefer managed.deinit();
        try managed.set(v);
        return Self{ .value = managed };
    }

    pub fn deinit(self: *Self) void {
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
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        // BigInt division truncates toward zero (like JS)
        try result.divTrunc(&self.value, &other.value);
        return Self{ .value = result };
    }

    /// Exponentiation: self ^ exp.
    /// `exp` must fit in u64 (JS semantics: exponent is converted to u64 via ToUint64).
    pub fn pow(self: *const Self, exp: u64, alloc: std.mem.Allocator) !Self {
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        // Managed.pow(r, a, b): r = a ^ b (b is u64)
        std.math.big.int.Managed.pow(&result, &self.value, exp);
        return Self{ .value = result };
    }

    pub fn neg(self: *const Self, alloc: std.mem.Allocator) !Self {
        var result = try std.math.big.int.Managed.init(alloc);
        errdefer result.deinit();
        result.copySign(&self.value, -1);
        return Self{ .value = result };
    }

    // ---- comparison ----

    pub fn eq(self: *const Self, other: *const Self) bool {
        return self.value.order(&other.value) == .eq;
    }

    pub fn order(self: *const Self, other: *const Self) std.math.Order {
        return self.value.order(&other.value);
    }

    // ---- conversion ----

    pub fn toI64(self: *const Self) !i64 {
        return try self.value.to(i64);
    }

    pub fn toU64(self: *const Self) !u64 {
        return try self.value.to(u64);
    }

    pub fn toString(self: *const Self, alloc: std.mem.Allocator) ![]u8 {
        return try self.value.toString(alloc, 10, false);
    }

    pub fn format(self: *const Self, writer: anytype) !void {
        try self.value.format(writer, 10, false);
    }
};
