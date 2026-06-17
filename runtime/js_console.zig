//! JS Console method implementations for Zig.

const std = @import("std");

/// Console.log — prints a formatted string to stdout.
pub fn log(msg: []const u8) void {
    std.debug.print("{s}\n", .{msg});
}

/// Console.error — prints to stderr with [ERROR] prefix.
pub fn err(msg: []const u8) void {
    std.debug.print("[ERROR] {s}\n", .{msg});
}

/// Console.warn — prints to stderr with [WARN] prefix.
pub fn warn(msg: []const u8) void {
    std.debug.print("[WARN] {s}\n", .{msg});
}

test "log" {
    log("test message");
}

test "error" {
    err("test error");
}

test "warn" {
    warn("test warning");
}
