//! Return type for C ABI string functions.
//!
//! ## Sign-bit convention
//! `len >= 0` → normal string of that length (arena-allocated, owned by Zig).
//! `len <  0` → panic occurred (async error propagated via CABI wrapper).
//!             The caller MUST NOT dereference `ptr` when `len < 0`.  Rust
//!             bridge macro converts this to `Result::Err(...)`.
//!
//! Zig side: `StrRet.from(slice)` / `StrRet.from_panic()`
//! Rust side: `#[repr(C)] struct __JsStr { ptr: *const u8, len: isize }`

const std = @import("std");

/// C-ABI-compatible string-or-panic return type.
pub const StrRet = extern struct {
    ptr: [*c]const u8,
    /// Positive → string length.  Negative → panic flag.
    len: isize,

    /// Build a StrRet from a `[]const u8` (arena-allocated slice).
    pub fn from(s: []const u8) StrRet {
        return StrRet{ .ptr = s.ptr, .len = @intCast(s.len) };
    }

    /// Build a StrRet signalling that an async error occurred.
    /// `len = -1` tells the Rust bridge to return `Err(...)`.
    pub fn from_panic() StrRet {
        return StrRet{ .ptr = null, .len = -1 };
    }

    /// Check if this StrRet signals a panic (async error).
    pub fn is_panic(self: StrRet) bool {
        return self.len < 0;
    }

    /// Convert to a `[]const u8` slice (caller must ensure pointer validity).
    /// Returns empty string for panic or null pointers.
    pub fn toSlice(self: StrRet) []const u8 {
        if (self.is_panic() or self.ptr == null) {
            return "";
        }
        return self.ptr[0..@intCast(self.len)];
    }
};
