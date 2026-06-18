//! js_runtime — Tier 3 runtime library for js2rust
//! Provides JS-like APIs for generated Zig code.

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

// Re-export commonly used types at top level for convenience.
pub const JsValue = jsvalue.JsValue;
pub const JsAny = jsany.JsAny;
pub const JsArrayList = jsany.JsArrayList;
pub const JsObjectMap = jsany.JsObjectMap;
