//! js_runtime — Tier 3 runtime library for js2rust
//! Provides JS-like APIs for generated Zig code.
//! All allocating functions accept `alloc: std.mem.Allocator` as first parameter.

pub const js_string = @import("js_runtime/js_string.zig");
pub const js_console = @import("js_runtime/js_console.zig");
pub const js_json = @import("js_runtime/js_json.zig");
pub const js_array = @import("js_runtime/js_array.zig");
pub const jsvalue = @import("js_runtime/jsvalue.zig");

test {
    _ = js_string;
    _ = js_console;
    _ = js_json;
    _ = js_array;
}
