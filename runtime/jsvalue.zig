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

    pub fn isBool(self: JsValue) bool {
        return self == .bool;
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

    pub fn typeName(self: JsValue) []const u8 {
        return switch (self) {
            .int => "number",
            .float => "number",
            .bool => "boolean",
            .string => "string",
            .null => "object",
            .undefined => "undefined",
        };
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
            .float => |v| @as(i64, @intFromFloat(v)),
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

    // --- arithmetic (JS semantics) ---

    /// add: if either operand is a string, concatenate; otherwise numeric add.
    pub fn add(self: JsValue, other: JsValue, alloc: Allocator) JsValue {
        if (self == .string or other == .string) {
            const self_s = self.asString(alloc);
            const other_s = other.asString(alloc);
            const result = std.fmt.allocPrint(alloc, "{s}{s}", .{ self_s, other_s }) catch "";
            return .{ .string = result };
        }
        if (self == .int and other == .int) {
            return .{ .int = self.int + other.int };
        }
        return .{ .float = self.asF64() + other.asF64() };
    }

    pub fn sub(self: JsValue, other: JsValue) JsValue {
        return .{ .float = self.asF64() - other.asF64() };
    }

    pub fn mul(self: JsValue, other: JsValue) JsValue {
        return .{ .float = self.asF64() * other.asF64() };
    }

    pub fn div(self: JsValue, other: JsValue) JsValue {
        const denom = other.asF64();
        if (denom == 0.0) return .{ .float = std.math.inf(f64) };
        return .{ .float = self.asF64() / denom };
    }

    pub fn rem(self: JsValue, other: JsValue) JsValue {
        return .{ .float = @mod(self.asF64(), other.asF64()) };
    }

    // --- unary ---

    pub fn neg(self: JsValue) JsValue {
        return switch (self) {
            .int => |v| .{ .int = -v },
            else => .{ .float = -self.asF64() },
        };
    }

    pub fn not(self: JsValue) JsValue {
        return .{ .bool = !self.asBool() };
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
        // Cross-type: coerce both to f64 and compare (JS loose == semantics).
        // This handles: int vs float, int vs string, int vs bool,
        //               float vs string, float vs bool, string vs bool.
        return self.asF64() == other.asF64();
    }

    /// Strict equality (===): same type AND same value. No coercion.
    pub fn strictEq(self: JsValue, other: JsValue) bool {
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

    pub fn lt(self: JsValue, other: JsValue) bool {
        return self.asF64() < other.asF64();
    }

    pub fn le(self: JsValue, other: JsValue) bool {
        return self.asF64() <= other.asF64();
    }

    pub fn gt(self: JsValue, other: JsValue) bool {
        return self.asF64() > other.asF64();
    }

    pub fn ge(self: JsValue, other: JsValue) bool {
        return self.asF64() >= other.asF64();
    }
};
