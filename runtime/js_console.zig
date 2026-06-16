//! JS Console method implementations for Zig.

const std = @import("std");

/// Console.log — prints a formatted string to stdout.
pub fn log(msg: []const u8) !void {
    const stdout = std.io.getStdOut().writer();
    try stdout.print("{s}\n", .{msg});
}

/// Console.error — prints to stderr with [ERROR] prefix.
pub fn err(msg: []const u8) !void {
    const stderr = std.io.getStdErr().writer();
    try stderr.print("[ERROR] {s}\n", .{msg});
}

/// Console.warn — prints to stderr with [WARN] prefix.
pub fn warn(msg: []const u8) !void {
    const stderr = std.io.getStdErr().writer();
    try stderr.print("[WARN] {s}\n", .{msg});
}

test "log" {
    try log("test message");
}

test "error" {
    try err("test error");
}

test "warn" {
    try warn("test warning");
}
