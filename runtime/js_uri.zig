//! js_uri — encodeURI / decodeURI / encodeURIComponent / decodeURIComponent for js2rust
//! Simplified percent-encoding for ASCII + UTF-8 bytes.

const std = @import("std");
const Allocator = std.mem.Allocator;

/// Percent-encode a string (encodeURIComponent).
/// Escapes all characters except: A-Z a-z 0-9 - _ . ! ~ * ' ( )
pub fn encodeURIComponent(alloc: Allocator, input: []const u8) ![]u8 {
    return encodeWithTable(alloc, input, isUnreservedComponent);
}

/// Percent-encode a string (encodeURI).
/// Escapes all characters except: A-Z a-z 0-9 ; , / ? : @ & = + $ - _ . ! ~ * ' ( ) #
pub fn encodeURI(alloc: Allocator, input: []const u8) ![]u8 {
    return encodeWithTable(alloc, input, isUnreservedURI);
}

/// Internal: percent-encode using a custom isUnreserved function.
fn encodeWithTable(alloc: Allocator, input: []const u8, comptime is_unreserved: anytype) ![]u8 {
    var encoded_len: usize = 0;
    for (input) |byte| {
        if (is_unreserved(byte)) {
            encoded_len += 1;
        } else {
            encoded_len += 3; // %XX
        }
    }

    const result = try alloc.alloc(u8, encoded_len);
    var pos: usize = 0;
    for (input) |byte| {
        if (is_unreserved(byte)) {
            result[pos] = byte;
            pos += 1;
        } else {
            const hex = "0123456789ABCDEF";
            result[pos] = '%';
            result[pos + 1] = hex[byte >> 4];
            result[pos + 2] = hex[byte & 0xF];
            pos += 3;
        }
    }
    return result;
}

/// Decode a percent-encoded string (decodeURIComponent).
/// Returns error.InvalidUriEncoding if invalid percent sequences.
pub fn decodeURIComponent(alloc: Allocator, input: []const u8) ![]u8 {
    return decodePercent(alloc, input);
}

/// Decode a percent-encoded string (decodeURI).
/// Same as decodeURIComponent for simplified implementation.
pub fn decodeURI(alloc: Allocator, input: []const u8) ![]u8 {
    return decodePercent(alloc, input);
}

/// Internal: decode percent-encoded string.
fn decodePercent(alloc: Allocator, input: []const u8) ![]u8 {
    // Count decoded length
    var decoded_len: usize = 0;
    var j: usize = 0;
    while (j < input.len) {
        if (input[j] == '%') {
            decoded_len += 1;
            j += 3;
        } else {
            decoded_len += 1;
            j += 1;
        }
    }

    const result = try alloc.alloc(u8, decoded_len);
    errdefer alloc.free(result);
    var pos: usize = 0;
    var i: usize = 0;

    while (i < input.len) {
        if (input[i] == '%') {
            if (i + 2 >= input.len) return error.InvalidUriEncoding;
            const hi = hexDigit(input[i + 1]) orelse return error.InvalidUriEncoding;
            const lo = hexDigit(input[i + 2]) orelse return error.InvalidUriEncoding;
            result[pos] = (@as(u8, hi) << 4) | @as(u8, lo);
            pos += 1;
            i += 3;
        } else {
            result[pos] = input[i];
            pos += 1;
            i += 1;
        }
    }
    return result;
}

/// Characters that are NOT percent-encoded by encodeURIComponent
fn isUnreservedComponent(byte: u8) bool {
    return switch (byte) {
        'A'...'Z', 'a'...'z', '0'...'9', '-', '_', '.', '!', '~', '*', '\'', '(', ')' => true,
        else => false,
    };
}

/// Characters that are NOT percent-encoded by encodeURI (includes extra chars)
fn isUnreservedURI(byte: u8) bool {
    return switch (byte) {
        'A'...'Z', 'a'...'z', '0'...'9',
        '-', '_', '.', '!', '~', '*', '\'', '(', ')',
        ';', ',', '/', '?', ':', '@', '&', '=', '+', '$', '#' => true,
        else => false,
    };
}

fn hexDigit(c: u8) ?u4 {
    return switch (c) {
        '0'...'9' => @as(u4, @intCast(c - '0')),
        'A'...'F' => @as(u4, @intCast(c - 'A' + 10)),
        'a'...'f' => @as(u4, @intCast(c - 'a' + 10)),
        else => null,
    };
}

// ── Tests ──

test "encodeURIComponent basic" {
    const result = try encodeURIComponent(std.testing.allocator, "hello world");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello%20world", result);
}

test "encodeURIComponent special chars" {
    const result = try encodeURIComponent(std.testing.allocator, "a=b&c=d");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("a%3Db%26c%3Dd", result);
}

test "encodeURIComponent reserved" {
    const result = try encodeURIComponent(std.testing.allocator, "hello-world_123.ABC");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello-world_123.ABC", result);
}

test "decodeURIComponent basic" {
    const result = try decodeURIComponent(std.testing.allocator, "hello%20world");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello world", result);
}

test "decodeURIComponent roundtrip" {
    const original = "a=b&c=d";
    const encoded = try encodeURIComponent(std.testing.allocator, original);
    defer std.testing.allocator.free(encoded);
    const decoded = try decodeURIComponent(std.testing.allocator, encoded);
    defer std.testing.allocator.free(decoded);
    try std.testing.expectEqualStrings(original, decoded);
}

test "encodeURI basic" {
    const result = try encodeURI(std.testing.allocator, "hello world");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello%20world", result);
}

test "encodeURI reserved chars" {
    // encodeURI preserves: ; , / ? : @ & = + $ #
    const result = try encodeURI(std.testing.allocator, ";/?:@&=+$#");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings(";/?:@&=+$#", result);
}

test "encodeURI escapes spaces" {
    const result = try encodeURI(std.testing.allocator, "a b c");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("a%20b%20c", result);
}

test "decodeURI basic" {
    const result = try decodeURI(std.testing.allocator, "hello%20world");
    defer std.testing.allocator.free(result);
    try std.testing.expectEqualStrings("hello world", result);
}

test "decodeURI roundtrip" {
    const original = "hello world";
    const encoded = try encodeURI(std.testing.allocator, original);
    defer std.testing.allocator.free(encoded);
    const decoded = try decodeURI(std.testing.allocator, encoded);
    defer std.testing.allocator.free(decoded);
    try std.testing.expectEqualStrings(original, decoded);
}
