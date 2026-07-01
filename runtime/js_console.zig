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

/// Print a message with an optional prefix, using the appropriate format specifier
/// based on the runtime type of `msg`.
fn printMsg(prefix: []const u8, msg: anytype) void {
    const T = @TypeOf(msg);
    if (comptime isStringType(T)) {
        std.debug.print("{s}{s}\n", .{ prefix, msg });
    } else {
        switch (@typeInfo(T)) {
            .int, .comptime_int => std.debug.print("{s}{d}\n", .{ prefix, msg }),
            .float, .comptime_float => std.debug.print("{s}{d}\n", .{ prefix, msg }),
            .bool => std.debug.print("{s}{}\n", .{ prefix, msg }),
            else => std.debug.print("{s}{any}\n", .{ prefix, msg }),
        }
    }
}

/// Console.log — prints a formatted message to stdout.
pub fn log(msg: anytype) void {
    printMsg("", msg);
}

/// Console.error — prints to stderr with [ERROR] prefix.
pub fn err(msg: anytype) void {
    printMsg("[ERROR] ", msg);
}

/// Console.warn — prints to stderr with [WARN] prefix.
pub fn warn(msg: anytype) void {
    printMsg("[WARN] ", msg);
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

test "error" {
    err("test error");
}

test "warn" {
    warn("test warning");
}
