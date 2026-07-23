//! js_uri — encodeURI / decodeURI / encodeURIComponent / decodeURIComponent for js2rust
//! Simplified percent-encoding for ASCII + UTF-8 bytes.

const std = @import("std");
const Allocator = std.mem.Allocator;
const js_number = @import("js_number.zig");

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
    // Count decoded length (validate %-sequences so we don't over-allocate)
    var decoded_len: usize = 0;
    var j: usize = 0;
    while (j < input.len) {
        if (input[j] == '%') {
            if (j + 2 >= input.len) return error.InvalidUriEncoding;
            _ = hexDigit(input[j + 1]) orelse return error.InvalidUriEncoding;
            _ = hexDigit(input[j + 2]) orelse return error.InvalidUriEncoding;
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

/// parseInt: parse an integer from a string, returning f64 to support NaN.
/// Implements JS parseInt semantics: radix 2–36, "0x" prefix detection,
/// stops at first invalid char, returns NaN if no valid digits found.
/// Accepts anytype to support string, f64, i64, JsAny inputs.
pub fn parseInt(value: anytype, radix: ?i64) f64 {
    const T = @TypeOf(value);
    // Fast path: already a string slice
    if (T == []const u8) return parseIntStr(value, radix);
    // String literals: *const [N:0]u8 → coerce to []const u8
    if (switch (@typeInfo(T)) {
        .pointer => |p| switch (p.size) {
            .one => switch (@typeInfo(p.child)) {
                .array => |a| a.child == u8,
                else => false,
            },
            else => false,
        },
        else => false,
    }) return parseIntStr(value, radix);
    // Float: format to buffer, then parse
    if (T == f64 or T == comptime_float) {
        var buf: [64]u8 = undefined;
        const s = std.fmt.bufPrint(&buf, "{d}", .{value}) catch return std.math.nan(f64);
        return parseIntStr(s, radix);
    }
    // Int: format to buffer, then parse
    if (T == i64 or T == comptime_int) {
        var buf: [64]u8 = undefined;
        const s = std.fmt.bufPrint(&buf, "{d}", .{value}) catch return std.math.nan(f64);
        return parseIntStr(s, radix);
    }
    return std.math.nan(f64);
}

/// Convert a single character to its digit value for the given base (2–36).
fn digitValue(c: u8, base: u8) ?u8 {
    const d: u8 = switch (c) {
        '0'...'9' => c - '0',
        'A'...'Z' => c - 'A' + 10,
        'a'...'z' => c - 'a' + 10,
        else => return null,
    };
    if (d < base) return d;
    return null;
}

fn parseIntStr(s: []const u8, radix: ?i64) f64 {
    var i: usize = 0;

    // Skip leading whitespace
    while (i < s.len and std.ascii.isWhitespace(s[i])) : (i += 1) {}

    // Handle sign
    var neg = false;
    if (i < s.len and s[i] == '-') {
        neg = true;
        i += 1;
    } else if (i < s.len and s[i] == '+') {
        i += 1;
    }

    // Determine the base
    var base: u8 = 10;
    if (radix) |r| {
        // Radix must be 2..36; outside that range → NaN
        if (r < 2 or r > 36) return std.math.nan(f64);
        base = @intCast(r);
    }

    // Handle "0x" / "0X" prefix for hex when radix is 0, 16, or unspecified
    if (i + 1 < s.len and s[i] == '0' and (s[i + 1] == 'x' or s[i + 1] == 'X')) {
        if (radix == null or radix == 0 or radix == 16) {
            base = 16;
            i += 2;
        }
    }

    // Parse digits using i128 for exact integer arithmetic, then convert to f64.
    // This matches JS spec: parseInt parses an integer (exact), then converts to Number (f64).
    // Using f64 accumulation causes rounding errors at each step for large numbers.
    var result: i128 = 0;
    var found_digit = false;
    var use_f64 = false;
    var result_f64: f64 = 0;
    const base_i128: i128 = @intCast(base);
    const base_f64: f64 = @floatFromInt(base);
    while (i < s.len) : (i += 1) {
        const d = digitValue(s[i], base) orelse break;
        if (!use_f64) {
            // Check if next multiply+add would overflow i128
            if (result > @divTrunc(std.math.maxInt(i128) - @as(i128, d), base_i128)) {
                // Overflow imminent — switch to f64 for remaining digits
                use_f64 = true;
                result_f64 = @as(f64, @floatFromInt(result));
                result_f64 = result_f64 * base_f64 + @as(f64, @floatFromInt(d));
            } else {
                result = result * base_i128 + @as(i128, d);
            }
        } else {
            result_f64 = result_f64 * base_f64 + @as(f64, @floatFromInt(d));
        }
        found_digit = true;
    }

    if (!found_digit) return std.math.nan(f64);
    const f: f64 = if (use_f64) result_f64 else @as(f64, @floatFromInt(result));
    return if (neg) -f else f;
}

/// parseFloat: parse a float from a string.
/// Delegates to js_number.parseFloat which handles exponents, Infinity, and NaN.
pub fn parseFloat(s: []const u8) f64 {
    return js_number.parseFloat(s);
}

/// isNaN stub: check if a float is NaN.
pub fn isNaN(v: f64) bool {
    return v != v;
}

/// isFinite stub: check if a float is finite (not Inf or NaN).
pub fn isFinite(v: f64) bool {
    return !isNaN(v) and v != std.math.inf(f64) and v != -std.math.inf(f64);
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

test "parseInt skips all whitespace types (P2-1)" {
    // Tab, newline, vertical tab, form feed, carriage return
    try std.testing.expectEqual(@as(f64, 42), parseInt("\t42", null));
    try std.testing.expectEqual(@as(f64, 42), parseInt("\n42", null));
    try std.testing.expectEqual(@as(f64, 42), parseInt("\r\n42", null));
    try std.testing.expectEqual(@as(f64, 42), parseInt("  \t 42", null));
}

test "parseFloat delegates to js_number (P2-2)" {
    // Exponents and Infinity are now handled via delegation to js_number.parseFloat
    try std.testing.expectEqual(@as(f64, 100000), parseFloat("1e5"));
    try std.testing.expectEqual(@as(f64, 3.14), parseFloat("3.14"));
    // Invalid input returns NaN (not 0.0 as in the old local implementation)
    try std.testing.expect(std.math.isNan(parseFloat("abc")));
}
