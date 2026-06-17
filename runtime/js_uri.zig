//! js_uri — encodeURIComponent / decodeURIComponent for js2rust
//! Simplified percent-encoding for ASCII + UTF-8 bytes.

const std = @import("std");

/// Percent-encode a string, escaping all characters except:
/// A-Z a-z 0-9 - _ . ! ~ * ' ( )
pub fn encodeURIComponent(alloc: std.mem.Allocator, input: []const u8) ![]u8 {
    // First pass: count how many bytes need encoding
    var encoded_len: usize = 0;
    for (input) |byte| {
        if (isUnreserved(byte)) {
            encoded_len += 1;
        } else {
            encoded_len += 3; // %XX
        }
    }

    var result = try alloc.alloc(u8, encoded_len);
    var pos: usize = 0;
    for (input) |byte| {
        if (isUnreserved(byte)) {
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

/// Decode a percent-encoded string.
/// Returns error.InvalidUriEncoding if the input contains invalid percent sequences.
pub fn decodeURIComponent(alloc: std.mem.Allocator, input: []const u8) ![]u8 {
    // Count decoded length: each %XX → 1 byte, everything else → 1 byte
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

    var result = try alloc.alloc(u8, decoded_len);
    errdefer alloc.free(result);
    var pos: usize = 0;
    var i: usize = 0;

    while (i < input.len) {
        if (input[i] == '%') {
            // Need at least 2 more chars after %
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
fn isUnreserved(byte: u8) bool {
    return switch (byte) {
        'A'...'Z', 'a'...'z', '0'...'9', '-', '_', '.', '!', '~', '*', '\'', '(', ')' => true,
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
