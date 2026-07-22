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

// ── Tests ──

const testing = std.testing;

test "localeCompare: equal strings return 0" {
    try testing.expectEqual(@as(i64, 0), localeCompare("hello", "hello"));
    try testing.expectEqual(@as(i64, 0), localeCompare("", ""));
    try testing.expectEqual(@as(i64, 0), localeCompare("abc123", "abc123"));
}

test "localeCompare: less than returns -1" {
    try testing.expectEqual(@as(i64, -1), localeCompare("apple", "banana"));
    try testing.expectEqual(@as(i64, -1), localeCompare("", "a"));
    try testing.expectEqual(@as(i64, -1), localeCompare("abc", "abd"));
}

test "localeCompare: greater than returns 1" {
    try testing.expectEqual(@as(i64, 1), localeCompare("banana", "apple"));
    try testing.expectEqual(@as(i64, 1), localeCompare("a", ""));
    try testing.expectEqual(@as(i64, 1), localeCompare("abd", "abc"));
}

test "normalize: returns copy of input (simplified)" {
    const allocator = testing.allocator;
    const input = "héllo wörld";
    const result_nfc = try normalize(allocator, input, "NFC");
    defer allocator.free(result_nfc);
    try testing.expectEqualStrings(input, result_nfc);

    const result_nfd = try normalize(allocator, input, "NFD");
    defer allocator.free(result_nfd);
    try testing.expectEqualStrings(input, result_nfd);

    const result_nfkc = try normalize(allocator, input, "NFKC");
    defer allocator.free(result_nfkc);
    try testing.expectEqualStrings(input, result_nfkc);

    const result_nfkd = try normalize(allocator, input, "NFKD");
    defer allocator.free(result_nfkd);
    try testing.expectEqualStrings(input, result_nfkd);
}

test "normalize: empty string" {
    const allocator = testing.allocator;
    const result = try normalize(allocator, "", "NFC");
    defer allocator.free(result);
    try testing.expectEqualStrings("", result);
}

test "normalize: result is a copy, not the same pointer" {
    const allocator = testing.allocator;
    const input = "test";
    const result = try normalize(allocator, input, "NFC");
    defer allocator.free(result);
    try testing.expectEqualStrings(input, result);
    try testing.expect(result.ptr != input.ptr);
}

test "toLocaleUpper: lowercase to uppercase" {
    const allocator = testing.allocator;
    const result = try toLocaleUpper(allocator, "hello world");
    defer allocator.free(result);
    try testing.expectEqualStrings("HELLO WORLD", result);
}

test "toLocaleUpper: already uppercase stays same" {
    const allocator = testing.allocator;
    const result = try toLocaleUpper(allocator, "HELLO");
    defer allocator.free(result);
    try testing.expectEqualStrings("HELLO", result);
}

test "toLocaleUpper: mixed case" {
    const allocator = testing.allocator;
    const result = try toLocaleUpper(allocator, "HeLLo");
    defer allocator.free(result);
    try testing.expectEqualStrings("HELLO", result);
}

test "toLocaleUpper: empty string" {
    const allocator = testing.allocator;
    const result = try toLocaleUpper(allocator, "");
    defer allocator.free(result);
    try testing.expectEqualStrings("", result);
}

test "toLocaleLower: uppercase to lowercase" {
    const allocator = testing.allocator;
    const result = try toLocaleLower(allocator, "HELLO WORLD");
    defer allocator.free(result);
    try testing.expectEqualStrings("hello world", result);
}

test "toLocaleLower: already lowercase stays same" {
    const allocator = testing.allocator;
    const result = try toLocaleLower(allocator, "hello");
    defer allocator.free(result);
    try testing.expectEqualStrings("hello", result);
}

test "toLocaleLower: mixed case" {
    const allocator = testing.allocator;
    const result = try toLocaleLower(allocator, "HeLLo");
    defer allocator.free(result);
    try testing.expectEqualStrings("hello", result);
}

test "toLocaleLower: empty string" {
    const allocator = testing.allocator;
    const result = try toLocaleLower(allocator, "");
    defer allocator.free(result);
    try testing.expectEqualStrings("", result);
}
