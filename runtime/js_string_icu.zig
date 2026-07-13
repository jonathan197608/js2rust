//! JS String ICU-dependent method implementations for Zig.
//!
//! This module provides locale-sensitive String methods:
//! - localeCompare
//! - normalize (NFC/NFD/NFKC/NFKD)
//! - toLocaleUpperCase
//! - toLocaleLowerCase
//!
//! By default (when needs_icu = false), these use simplified implementations
//! that do not require ICU4X. When needs_icu = true, the project generator
//! overwrites this file with the ICU4X-based version that calls host_icu_*
//! C ABI functions provided by js2rust-bridge (native_icu.rs).

const std = @import("std");
const Allocator = std.mem.Allocator;
const JsAny = @import("jsany.zig").JsAny;

// ── Simplified (non-ICU) implementations ──
// These are used when needs_icu = false. When needs_icu = true, this file
// is overwritten with the ICU4X version that declares host_icu_* extern fns.

/// Import the basic toUpper/toLower for the simplified locale methods.
const js_string_internal = @import("js_string.zig");

/// Locale-sensitive string comparison (simplified: byte-wise comparison).
/// Returns -1 if self < other, 0 if equal, 1 if self > other.
pub fn localeCompare(self: []const u8, other: []const u8) i64 {
    return switch (std.mem.order(u8, self, other)) {
        .lt => -1,
        .eq => 0,
        .gt => 1,
    };
}

/// Normalize Unicode string (simplified: returns a copy of the input).
/// For proper Unicode normalization, enable the `icu` feature.
pub fn normalize(alloc: Allocator, s: []const u8, form: []const u8) ![]const u8 {
    _ = form;
    return try alloc.dupe(u8, s);
}

/// Convert string to locale-specific uppercase (simplified: uses ASCII toUpper).
pub fn toLocaleUpper(alloc: Allocator, s: []const u8) ![]const u8 {
    return js_string_internal.toUpper(alloc, s);
}

/// Convert string to locale-specific lowercase (simplified: uses ASCII toLower).
pub fn toLocaleLower(alloc: Allocator, s: []const u8) ![]const u8 {
    return js_string_internal.toLower(alloc, s);
}
