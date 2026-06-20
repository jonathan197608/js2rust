//! js_runtime — Tier 3 runtime library for js2rust
//! Provides JS-like APIs for generated Zig code.

const std = @import("std");
const Io = std.Io;

pub const js_string = @import("js_string.zig");
pub const js_console = @import("js_console.zig");
pub const js_json = @import("js_json.zig");
pub const js_array = @import("js_array.zig");
pub const js_object = @import("js_object.zig");
pub const js_number = @import("js_number.zig");
pub const js_date = @import("js_date.zig");
pub const js_error = @import("js_error.zig");
pub const js_map = @import("js_map.zig");
pub const js_set = @import("js_set.zig");
pub const js_regexp = @import("js_regexp.zig");
pub const js_uri = @import("js_uri.zig");
pub const jsvalue = @import("jsvalue.zig");
pub const jsany = @import("jsany.zig");

pub const js_typedarray = @import("js_typedarray.zig");

pub const js_promise = @import("js_promise.zig");
pub const Promise = js_promise.Promise;

// Re-export commonly used types at top level for convenience.
pub const JsValue = jsvalue.JsValue;
pub const JsAny = jsany.JsAny;
pub const JsArrayList = jsany.JsArrayList;
pub const JsObjectMap = jsany.JsObjectMap;
pub const undefined_value = jsany.undefined_value;

// ── Global Io for C ABI blocking wrappers ──────────────────────
// When async functions are exported via C ABI, the wrapper needs an Io
// instance to call io.async() / .await(). We use Io.Threaded (blocking,
// thread-pool based) so the C ABI call blocks until the async work completes.
// Heap-allocated to guarantee proper alignment for atomic fields.

var g_threaded: ?*std.Io.Threaded = null;
var g_io_allocator: ?std.mem.Allocator = null;

/// Initialize the global Io. Called from init_js2rust().
pub fn initIo(allocator: std.mem.Allocator) void {
    if (g_threaded != null) return;
    g_io_allocator = allocator;
    const t = allocator.create(std.Io.Threaded) catch @panic("initIo: out of memory");
    t.* = .init(allocator, .{});
    g_threaded = t;
}

/// Get the global Io instance for C ABI blocking wrappers.
pub fn getIo() Io {
    if (g_threaded) |t| {
        return t.io();
    }
    @panic("js_runtime: Io not initialized. Call initIo() first.");
}

/// Release the global Io. Called from deinit_js2rust().
pub fn deinitIo() void {
    if (g_threaded) |t| {
        t.deinit();
        if (g_io_allocator) |a| {
            a.destroy(t);
        }
    }
    g_threaded = null;
    g_io_allocator = null;
}
