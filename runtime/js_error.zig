//! JS Error constructor and message property for Zig.
//! Maps to Zig error unions naturally.

const std = @import("std");
const Allocator = std.mem.Allocator;
const js_allocator = @import("js_allocator.zig");

/// Thread-local storage for `throw new ErrorSubclass(msg)` pattern.
/// When JS code does `throw new TypeError("custom error")`, the throw
/// statement stores the error name and message here before breaking
/// to the try-catch labeled block with `error.JsThrow`.
/// The catch handler's `fromError()` reads these to construct a JsError
/// with the correct name and message.
threadlocal var last_throw_name: ?[]const u8 = null;
threadlocal var last_throw_msg: ?[]const u8 = null;

/// Store the name and message of a `throw new Error(msg)` expression.
/// Called before the actual `break :label error.JsThrow` in generated code.
pub fn setLastThrow(name: []const u8, msg: []const u8) void {
    last_throw_name = name;
    last_throw_msg = msg;
}


/// Error "class" — wraps name, message and stack strings.
/// In JS, caught errors have `.name`, `.message` and `.stack` properties.
pub const JsError = struct {
    name: []const u8,
    message: []const u8,
    stack: []const u8,

    pub fn init(alloc: Allocator, name: []const u8, msg: []const u8) !JsError {
        const name_copy = try alloc.dupe(u8, name);
        errdefer alloc.free(name_copy);
        const msg_copy = try alloc.dupe(u8, msg);
        errdefer alloc.free(msg_copy);
        const stack = try std.fmt.allocPrint(alloc, "{s}: {s}", .{ name, msg });
        return JsError{
            .name = name_copy,
            .message = msg_copy,
            .stack = stack,
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
        // For `throw new Error(msg)`, the name and message were stored
        // in thread-local vars before the throw. Use them if available.
        if (err == error.JsThrow) {
            if (last_throw_name != null and last_throw_msg != null) {
                const name_copy = try alloc.dupe(u8, last_throw_name.?);
                errdefer alloc.free(name_copy);
                const msg_copy = try alloc.dupe(u8, last_throw_msg.?);
                errdefer alloc.free(msg_copy);
                const stack = try std.fmt.allocPrint(alloc, "{s}: {s}", .{ last_throw_name.?, last_throw_msg.? });
                // Reset thread-local vars to avoid stale data
                last_throw_name = null;
                last_throw_msg = null;
                return JsError{
                    .name = name_copy,
                    .message = msg_copy,
                    .stack = stack,
                };
            }
        }
        const info = errorInfo(err);
        const name_copy = try alloc.dupe(u8, info.name);
        errdefer alloc.free(name_copy);
        const msg_copy = try alloc.dupe(u8, info.message);
        errdefer alloc.free(msg_copy);
        const stack = try std.fmt.allocPrint(alloc, "{s}: {s}", .{ info.name, info.message });
        return JsError{
            .name = name_copy,
            .message = msg_copy,
            .stack = stack,
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
            // JSON.parse errors → SyntaxError (matching JS spec)
            error.SyntaxError,
            error.UnexpectedEndOfInput,
            error.InvalidJSON,
            error.EmptyInput,
            error.UnexpectedToken,
            error.InvalidNumber,
            error.MaxDepthExceeded,
            => .{ .name = "SyntaxError", .message = "Unexpected token" },
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
