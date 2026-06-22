//! Return type for C ABI string functions.
//!
//! ## Sign-bit convention
//! `len >= 0` → normal string of that length (arena-allocated, owned by Zig).
//! `len <  0` → panic occurred (async error propagated via CABI wrapper).
//!             `@abs(len)` bytes at `ptr` contain the error name (static, no free needed).
//!             Rust bridge macro converts this to `Result::Err(msg)`.
//!
//! Zig side: `StrRet.from(slice)` / `StrRet.from_panic(err)`
//! Rust side: `#[repr(C)] struct __JsStr { ptr: *const u8, len: isize }`

const std = @import("std");

/// C-ABI-compatible string-or-panic return type.
pub const StrRet = extern struct {
    ptr: [*c]const u8,
    /// Positive → string length.  Negative → panic flag, |len| = error name length.
    len: isize,

    /// Build a StrRet from a `[]const u8` (arena-allocated slice).
    pub fn from(s: []const u8) StrRet {
        return StrRet{ .ptr = s.ptr, .len = @intCast(s.len) };
    }

    /// Build a StrRet signalling an async error occurred.
    /// Uses `@errorName(err)` — a compile-time static string in the binary's
    /// data section.  Zero allocation, zero free needed.
    pub fn from_panic(err: anyerror) StrRet {
        const name: [:0]const u8 = @errorName(err);
        return StrRet{ .ptr = name.ptr, .len = -@as(isize, @intCast(name.len)) };
    }

    /// Check if this StrRet signals a panic (async error).
    pub fn is_panic(self: StrRet) bool {
        return self.len < 0;
    }

    /// Get the error message from a panic StrRet, or null if not a panic.
    pub fn panic_msg(self: StrRet) ?[]const u8 {
        if (!self.is_panic() or self.ptr == null) return null;
        const msg_len = @as(usize, @intCast(-self.len));
        return self.ptr[0..msg_len];
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
