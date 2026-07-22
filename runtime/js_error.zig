//! JS Error constructor and message property for Zig.
//! Maps to Zig error unions naturally.

const std = @import("std");
const Allocator = std.mem.Allocator;
const js_allocator = @import("js_allocator.zig");

/// Error "class" — wraps name, message and stack strings.
/// In JS, caught errors have `.name`, `.message` and `.stack` properties.
pub const JsError = struct {
    name: []const u8,
    message: []const u8,
    stack: []const u8,

    pub fn init(alloc: Allocator, name: []const u8, msg: []const u8) !JsError {
        return JsError{
            .name = try alloc.dupe(u8, name),
            .message = try alloc.dupe(u8, msg),
            .stack = try std.fmt.allocPrint(alloc, "{s}: {s}", .{ name, msg }),
        };
    }

    pub fn deinit(self: JsError, alloc: Allocator) void {
        if (js_allocator.isNoOpFree(alloc)) return;
        // name and message may point into the stack allocation,
        // so only free the stack string (which owns its own buffer).
        // If they were individually duped, free all three.
        // Safe approach: free all three since init() dupes name & messages.
        alloc.free(self.name);
        alloc.free(self.message);
        alloc.free(self.stack);
    }

    /// Custom format: prints "name: message" (matching Node.js console output).
    /// Without this, std.fmt defaults to `.{ .field = value, ... }`.
    pub fn format(self: JsError, w: *std.Io.Writer) std.Io.Writer.Error!void {
        try w.print("{s}: {s}", .{ self.name, self.message });
    }

    /// Construct a JsError from a Zig error union value.
    /// Maps known Zig errors to JS error names and messages; falls back to "Error".
    pub fn fromError(err: anyerror, alloc: Allocator) !JsError {
        const info = errorInfo(err);
        return JsError{
            .name = try alloc.dupe(u8, info.name),
            .message = try alloc.dupe(u8, info.message),
            .stack = try std.fmt.allocPrint(alloc, "{s}: {s}", .{ info.name, info.message }),
        };
    }

    /// Error metadata: JS name + standard message.
    const ErrorInfo = struct { name: []const u8, message: []const u8 };

    /// Map a Zig error value to JS error name and standard message.
    fn errorInfo(err: anyerror) ErrorInfo {
        return switch (err) {
            error.InvalidUriEncoding => .{ .name = "URIError", .message = "URI malformed" },
            error.DivisionByZero => .{ .name = "RangeError", .message = "BigInt division by zero" },
            error.ConstReassignment => .{ .name = "TypeError", .message = "Assignment to constant variable." },
            else => .{ .name = "Error", .message = @errorName(err) },
        };
    }
};

test "Error init" {
    const err = try JsError.init(std.testing.allocator, "URIError", "URI malformed");
    defer err.deinit(std.testing.allocator);
    try std.testing.expectEqualStrings("URIError", err.name);
    try std.testing.expectEqualStrings("URI malformed", err.message);
    try std.testing.expectEqualStrings("URIError: URI malformed", err.stack);
}

test "Error fromError" {
    const err = try JsError.fromError(error.InvalidUriEncoding, std.testing.allocator);
    defer err.deinit(std.testing.allocator);
    try std.testing.expectEqualStrings("URIError", err.name);
    try std.testing.expectEqualStrings("URI malformed", err.message);
}
