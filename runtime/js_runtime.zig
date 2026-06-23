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

// ── Object spread/merge helpers ─────────────────────────────────
// These provide compile-time merging of anonymous structs,
// used by the codegen for { ...a, ...b, c: 1 } syntax.

/// Compute the merged struct type for two anonymous structs.
/// Fields from B are concatenated after fields from A.
/// Duplicate field names will cause a compile error.
pub fn SpreadMerge(comptime A: type, comptime B: type) type {
    const a_info = @typeInfo(A).@"struct";
    const b_info = @typeInfo(B).@"struct";
    return @Type(.{ .@"struct" = .{
        .layout = .auto,
        .fields = a_info.fields ++ b_info.fields,
        .decls = &.{},
        .is_tuple = false,
    } });
}

/// Merge two anonymous structs at compile time.
/// Returns a new struct with all fields from `a` followed by all fields from `b`.
/// Duplicate field names across `a` and `b` will cause a compile error.
pub fn spreadMerge(a: anytype, b: anytype) SpreadMerge(@TypeOf(a), @TypeOf(b)) {
    const A = @TypeOf(a);
    const B = @TypeOf(b);
    if (@typeInfo(A) != .@"struct") {
        @compileError("spreadMerge: first argument must be an anonymous struct, got " ++ @typeName(A));
    }
    if (@typeInfo(B) != .@"struct") {
        @compileError("spreadMerge: second argument must be an anonymous struct, got " ++ @typeName(B));
    }

    var result: SpreadMerge(A, B) = undefined;
    inline for (@typeInfo(A).@"struct".fields) |f| {
        @field(result, f.name) = @field(a, f.name);
    }
    inline for (@typeInfo(B).@"struct".fields) |f| {
        @field(result, f.name) = @field(b, f.name);
    }
    return result;
}
