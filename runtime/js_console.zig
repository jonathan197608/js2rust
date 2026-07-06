//! JS Console method implementations for Zig.

const std = @import("std");

/// Check if a type is a string type ([]const u8, []u8, *const [N:0]u8, etc.)
fn isStringType(comptime T: type) bool {
    return switch (@typeInfo(T)) {
        .pointer => |p| switch (p.size) {
            .slice => p.child == u8,
            .one => switch (@typeInfo(p.child)) {
                .array => |a| a.child == u8,
                else => false,
            },
            else => false,
        },
        else => false,
    };
}

/// Print a single value (no prefix, no newline) using the appropriate format specifier.
fn printValue(msg: anytype) void {
    const T = @TypeOf(msg);
    if (comptime isStringType(T)) {
        std.debug.print("{s}", .{msg});
    } else {
        switch (@typeInfo(T)) {
            .int, .comptime_int => std.debug.print("{d}", .{msg}),
            .float, .comptime_float => std.debug.print("{d}", .{@as(f64, msg)}),
            .bool => std.debug.print("{}", .{msg}),
            else => std.debug.print("{any}", .{msg}),
        }
    }
}

/// Print a message with an optional prefix, using the appropriate format specifier
/// based on the runtime type of `msg`.
fn printMsg(prefix: []const u8, msg: anytype) void {
    std.debug.print("{s}", .{prefix});
    printValue(msg);
    std.debug.print("\n", .{});
}

/// Console.log — prints a single argument to stderr (Zig debug).
pub fn log(msg: anytype) void {
    printMsg("", msg);
}

/// Console.log with multiple arguments — JS joins args with spaces.
/// Usage: js_console.logMulti(.{ arg1, arg2, ... });
pub fn logMulti(args: anytype) void {
    const fields = std.meta.fields(@TypeOf(args));
    inline for (fields, 0..) |field, i| {
        if (i > 0) std.debug.print(" ", .{});
        printValue(@field(args, field.name));
    }
    std.debug.print("\n", .{});
}

/// Console.error — prints to stderr with [ERROR] prefix.
pub fn err(msg: anytype) void {
    printMsg("[ERROR] ", msg);
}

/// Console.error with multiple arguments.
pub fn errMulti(args: anytype) void {
    const fields = std.meta.fields(@TypeOf(args));
    inline for (fields, 0..) |field, i| {
        if (i > 0) std.debug.print(" ", .{});
        printValue(@field(args, field.name));
    }
    std.debug.print("\n", .{});
}

/// Console.warn — prints to stderr with [WARN] prefix.
pub fn warn(msg: anytype) void {
    printMsg("[WARN] ", msg);
}

/// Console.warn with multiple arguments.
pub fn warnMulti(args: anytype) void {
    const fields = std.meta.fields(@TypeOf(args));
    inline for (fields, 0..) |field, i| {
        if (i > 0) std.debug.print(" ", .{});
        printValue(@field(args, field.name));
    }
    std.debug.print("\n", .{});
}

test "log string" {
    log("test message");
}

test "log int" {
    log(42);
}

test "log float" {
    log(3.14);
}

test "log bool" {
    log(true);
}

test "logMulti" {
    logMulti(.{ "PI:", 3.14159 });
}

test "error" {
    err("test error");
}

test "warn" {
    warn("test warning");
}
