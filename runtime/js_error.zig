//! JS Error constructor and message property for Zig.
//! Maps to Zig error unions naturally.

const std = @import("std");
const Allocator = std.mem.Allocator;

/// Error "class" — wraps a message string.
pub const JsError = struct {
    message: []const u8,

    pub fn init(alloc: Allocator, msg: []const u8) !JsError {
        return JsError{
            .message = try alloc.dupe(u8, msg),
        };
    }

    pub fn deinit(self: JsError, alloc: Allocator) void {
        alloc.free(self.message);
    }
};

test "Error init" {
    const err = try JsError.init(std.testing.allocator, "something went wrong");
    defer err.deinit(std.testing.allocator);
    try std.testing.expectEqualStrings("something went wrong", err.message);
}
