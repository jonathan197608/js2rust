//! Return type for C ABI string functions.
//! Zig side: `{ .ptr = slice.ptr, .len = slice.len }`
//! Rust side: `#[repr(C)] struct JsStr { ptr: *const u8, len: usize }`

const std = @import("std");

/// C-ABI-compatible string return type.
/// Instead of returning `[*:0]u8` + `result_len: *usize` output parameter,
/// string-returning C ABI functions now return a `StrRet` by value.
pub const StrRet = extern struct {
    ptr: [*c]const u8,
    len: usize,

    /// Build a StrRet from a `[]const u8` (arena-allocated slice).
    pub fn from(s: []const u8) StrRet {
        return StrRet{ .ptr = s.ptr, .len = s.len };
    }

    /// Convert to a `[]const u8` slice (caller must ensure pointer validity).
    pub fn toSlice(self: StrRet) []const u8 {
        if (self.ptr == @as([*c]const u8, undefined) or self.ptr == 0) {
            return "";
        }
        return self.ptr[0..self.len];
    }
};
