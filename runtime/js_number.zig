//! JS Number static method implementations for Zig.
//! Operates on numeric values directly, no allocation needed.

const std = @import("std");
const js_allocator = @import("js_allocator.zig");
const JsAny = @import("jsany.zig").JsAny;
const js_date = @import("js_date.zig");

/// Number(x) — convert a value to a number (f64).
/// Simplified: handles string, i64, f64, bool, JsDate, and JsAny inputs.
pub fn constructor(value: anytype) f64 {
    const T = @TypeOf(value);
    if (T == f64) return value;
    if (T == i64) return @as(f64, @floatFromInt(value));
    if (T == bool) return if (value) 1.0 else 0.0;
    if (T == []const u8) return parseFloat(value);
    if (T == js_date.JsDate) return @as(f64, @floatFromInt(value.valueOf()));
    // Fallback for JsAny or other types: return NaN
    return std.math.nan(f64);
}

/// Number.isNaN — check if a value is NaN.
pub fn isNaN(val: f64) bool {
    return std.math.isNan(val);
}

/// Number.isFinite — check if a value is finite.
pub fn isFinite(val: f64) bool {
    return !std.math.isInf(val) and !std.math.isNan(val);
}

/// Number.isInteger — check if a value is an integer (safe within i64 range).
pub fn isInteger(val: f64) bool {
    if (std.math.isNan(val) or std.math.isInf(val)) return false;
    return @mod(val, 1.0) == 0.0;
}

/// JS parseInt — parse an integer from a string with JS semantics.
/// Handles leading whitespace, sign, 0x prefix, and stops at first
/// non-digit character (e.g. decimal point). Returns 0 for NaN (i64 can't
/// represent NaN).
pub fn parseInt(value: anytype, radix: ?i64) i64 {
    const T = @TypeOf(value);
    // Fast path: already a string slice
    if (T == []const u8) {
        return parseIntStr(value, radix);
    }
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
    }) {
        return parseIntStr(value, radix);
    }
    if (T == JsAny) {
        const s = value.asString(js_allocator.allocator());
        return parseIntStr(s, radix);
    }
    // For numeric/bool types, format to a buffer
    var buf: [64]u8 = undefined;
    const s = switch (@typeInfo(T)) {
        .int, .comptime_int => std.fmt.bufPrint(&buf, "{d}", .{value}) catch return 0,
        .float, .comptime_float => std.fmt.bufPrint(&buf, "{d}", .{value}) catch return 0,
        .bool => if (value) "true" else "false",
        else => std.fmt.bufPrint(&buf, "{any}", .{value}) catch return 0,
    };
    return parseIntStr(s, radix);
}

fn parseIntStr(s: []const u8, radix: ?i64) i64 {
    var i: usize = 0;
    const len = s.len;

    // Skip leading whitespace (JS: trimStart)
    while (i < len and std.ascii.isWhitespace(s[i])) {
        i += 1;
    }
    if (i >= len) return 0;

    // Handle sign
    var negative = false;
    if (s[i] == '+' or s[i] == '-') {
        negative = s[i] == '-';
        i += 1;
    }
    if (i >= len) return 0;

    // Determine effective radix.
    // JS spec: radix 0 or undefined → auto-detect (0x → 16, else 10).
    // Valid radix range is 2..36; out-of-range values are treated as 10
    // (matching V8 behavior for parseInt(str, 1) which returns NaN in JS
    // but our i64 return type can't express NaN — we return 0 instead).
    var r: u8 = 10;
    if (radix) |rd| {
        if (rd == 0) {
            // radix 0 = auto-detect (same as undefined)
        } else if (rd >= 2 and rd <= 36) {
            r = @intCast(rd);
        } else {
            // Invalid radix: JS returns NaN, we return 0
            return 0;
        }
    }

    // Auto-detect 0x prefix:
    //   - radix undefined/0 → auto-detect and set radix
    //   - radix 16 → strip prefix
    const radix_undefined = (radix == null or radix.? == 0);
    if (radix_undefined or r == 16) {
        if (i + 1 < len and s[i] == '0' and (s[i + 1] == 'x' or s[i + 1] == 'X')) {
            r = 16;
            i += 2;
        }
    }

    // Parse digits using i128 accumulator to avoid panic on overflow.
    // JS parseInt returns a Number (f64); when the parsed magnitude exceeds
    // the i64 range of our return type, we clamp to the nearest representable
    // i64 rather than @intCast-panicking on huge inputs like
    // parseInt("99999999999999999999").
    var result: i128 = 0;
    var overflow = false;
    var has_digit = false;
    const base_i128: i128 = @intCast(r);
    const mul_limit: i128 = @divTrunc(std.math.maxInt(i128), base_i128);
    while (i < len) {
        const c = s[i];
        const digit: u8 = blk: {
            if (c >= '0' and c <= '9') break :blk c - '0';
            if (c >= 'a' and c <= 'z') break :blk c - 'a' + 10;
            if (c >= 'A' and c <= 'Z') break :blk c - 'A' + 10;
            break :blk 255;
        };
        if (digit >= r) break;
        if (!overflow) {
            // Check if `result * base + digit` would overflow i128
            if (result > mul_limit) {
                overflow = true;
            } else {
                result = result * base_i128 + @as(i128, digit);
            }
        }
        has_digit = true;
        i += 1;
    }

    if (!has_digit) return 0;

    // Cast i128 → i64 with sign-aware clamping. The magnitude can be up to
    // 2^63 (= -minInt(i64)) when negative; for positive the cap is 2^63 - 1.
    const i64_max: i128 = std.math.maxInt(i64);
    const neg_max: i128 = i64_max + 1; // = 2^63, magnitude of minInt(i64)
    if (overflow) {
        return if (negative) std.math.minInt(i64) else std.math.maxInt(i64);
    }
    if (negative) {
        if (result >= neg_max) return std.math.minInt(i64);
        const v: i64 = @intCast(result);
        return -v;
    } else {
        if (result > i64_max) return std.math.maxInt(i64);
        const v: i64 = @intCast(result);
        return v;
    }
}

/// Number.parseFloat — parse a float from a string.
/// Per ECMA-262 §19.2.4, parses the longest prefix of the string that is
/// a valid float literal, and returns NaN if no valid prefix is found.
pub fn parseFloat(s: []const u8) f64 {
    // Trim leading whitespace (JS trimStart)
    var start: usize = 0;
    while (start < s.len and std.ascii.isWhitespace(s[start])) {
        start += 1;
    }
    const trimmed = s[start..];
    if (trimmed.len == 0) return std.math.nan(f64);

    // Find the longest valid float prefix by scanning forward.
    // Valid float syntax: [+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?
    // Also handle "Infinity" and "NaN" literals.
    if (std.mem.startsWith(u8, trimmed, "Infinity") or std.mem.startsWith(u8, trimmed, "+Infinity")) {
        return std.math.inf(f64);
    }
    if (std.mem.startsWith(u8, trimmed, "-Infinity")) {
        return -std.math.inf(f64);
    }

    var end: usize = 0;
    // Optional sign
    if (end < trimmed.len and (trimmed[end] == '+' or trimmed[end] == '-')) end += 1;

    const digits_start = end;
    var has_digits = false;
    // Integer part digits
    while (end < trimmed.len and std.ascii.isDigit(trimmed[end])) {
        end += 1;
        has_digits = true;
    }
    // Decimal point and fractional digits
    if (end < trimmed.len and trimmed[end] == '.') {
        end += 1;
        while (end < trimmed.len and std.ascii.isDigit(trimmed[end])) {
            end += 1;
            has_digits = true;
        }
    }
    if (!has_digits) {
        // Check for ".digits" pattern (digits_start points to '.')
        if (digits_start < trimmed.len and trimmed[digits_start] == '.') {
            // Already handled above, has_digits would be true if any digits followed
        }
        return std.math.nan(f64);
    }
    // Exponent part
    if (end < trimmed.len and (trimmed[end] == 'e' or trimmed[end] == 'E')) {
        const exp_start = end;
        end += 1;
        if (end < trimmed.len and (trimmed[end] == '+' or trimmed[end] == '-')) end += 1;
        var has_exp_digits = false;
        while (end < trimmed.len and std.ascii.isDigit(trimmed[end])) {
            end += 1;
            has_exp_digits = true;
        }
        if (!has_exp_digits) {
            // Roll back exponent part — 'e' without digits is not valid
            end = exp_start;
        }
    }

    const prefix = trimmed[0..end];
    return std.fmt.parseFloat(f64, prefix) catch std.math.nan(f64);
}

/// Number.isSafeInteger — check if a value is a safe integer (|v| <= 2^53-1).
pub fn isSafeInteger(val: f64) bool {
    if (std.math.isNan(val) or std.math.isInf(val)) return false;
    if (@mod(val, 1.0) != 0.0) return false;
    // Range check before @intFromFloat: finite-but-out-of-i64-range values
    // (e.g. 1e30) would otherwise panic in @intFromFloat. Comparing against
    // the safe-integer bounds early both avoids the panic and short-circuits
    // correctly per JS spec.
    const safe_max: f64 = 9007199254740991.0; // 2^53 - 1
    const safe_min: f64 = -9007199254740991.0; // -(2^53 - 1)
    if (val > safe_max or val < safe_min) return false;
    const v: i64 = @intFromFloat(val);
    return v >= -9007199254740991 and v <= 9007199254740991;
}

/// Number.prototype.toFixed — format a float to fixed-point string.
/// Uses inline for + comptimePrint to generate variable-precision format string.
pub fn toFixed(alloc: std.mem.Allocator, val: f64, digits: i64) ![]const u8 {
    // Handle special values
    if (std.math.isNan(val)) return alloc.dupe(u8, "NaN");
    if (std.math.isInf(val)) {
        return if (val > 0) alloc.dupe(u8, "Infinity") else alloc.dupe(u8, "-Infinity");
    }
    // Handle -0: JS requires "0" not "-0"
    if (val == 0.0) return try toFixed(alloc, 0.0, digits);
    // ECMA-262 §22.1.3.3: Throw RangeError if digits < 0 or digits > 100
    if (digits < 0 or digits > 100) return error.RangeError;
    const d: usize = @intCast(digits);

    var buf: [512]u8 = undefined;
    // Use inline for to generate all precision cases at comptime
    inline for (0..21) |p| {
        if (d == p) {
            const format = comptime std.fmt.comptimePrint("{{d:.{d}}}", .{p});
            const s = try std.fmt.bufPrint(&buf, format, .{val});
            return alloc.dupe(u8, s);
        }
    }
    // Fallback for digits 21-100: format with max comptime precision (20),
    // then compute additional digits via successive multiplication of the
    // fractional remainder.  Pre-fix: padded zeros after position 20, so
    // (0.1).toFixed(30) gave "0.100000000000000000000000000000" instead of
    // "0.100000000000000005551115123126".
    const s = try std.fmt.bufPrint(&buf, "{d:.20}", .{val});
    const dot_idx = std.mem.indexOfScalar(u8, s, '.') orelse {
        return alloc.dupe(u8, s);
    };
    // Extract the integer part and existing fractional digits.
    const int_part = s[0..dot_idx];
    const existing_frac = s[dot_idx + 1 ..];
    // Parse the existing fractional digits as f64 remainder.
    const remainder: f64 = std.fmt.parseFloat(f64, existing_frac) catch 0.0;
    // Compute additional digits needed beyond 20.
    if (d > existing_frac.len) {
        // Multiply remainder by 10^(additional) to get the extra digits.
        // We extract one digit at a time: digit = floor(rem * 10), rem -= digit/10.
        var extra_buf: [80]u8 = undefined; // max 80 extra (100 - 20)
        var extra_len: usize = 0;
        var rem = remainder;
        const needed = d - existing_frac.len;
        while (extra_len < needed and extra_len < extra_buf.len) {
            rem *= 10.0;
            const dgt: u8 = @intFromFloat(@floor(rem));
            extra_buf[extra_len] = '0' + @min(dgt, 9);
            extra_len += 1;
            rem -= @as(f64, @floatFromInt(dgt));
        }
        // Build result: int_part + "." + existing_frac + extra_digits
        const total_len = int_part.len + 1 + d; // +1 for dot, d total frac digits
        const result = try alloc.alloc(u8, total_len);
        @memcpy(result[0..int_part.len], int_part);
        result[int_part.len] = '.';
        @memcpy(result[int_part.len + 1 .. int_part.len + 1 + existing_frac.len], existing_frac);
        @memcpy(result[int_part.len + 1 + existing_frac.len .. int_part.len + 1 + existing_frac.len + extra_len], extra_buf[0..extra_len]);
        // Pad any remaining with '0' (shouldn't happen since d <= 100)
        if (int_part.len + 1 + d > int_part.len + 1 + existing_frac.len + extra_len) {
            @memset(result[int_part.len + 1 + existing_frac.len + extra_len ..], '0');
        }
        return result;
    }
    return alloc.dupe(u8, s);
}

/// Post-process a Zig `{e}` format string to insert '+' after 'e' for
/// positive exponents, matching the JS spec (e.g. "1.23e2" → "1.23e+2").
/// Negative exponents already carry '-' and are returned unchanged.
fn fixupExp(alloc: std.mem.Allocator, s: []const u8) ![]const u8 {
    const e_idx = std.mem.indexOfScalar(u8, s, 'e') orelse
        std.mem.indexOfScalar(u8, s, 'E') orelse return alloc.dupe(u8, s);
    if (e_idx + 1 < s.len and (s[e_idx + 1] == '-' or s[e_idx + 1] == '+')) {
        return alloc.dupe(u8, s);
    }
    // Insert '+' after 'e' for the unsigned positive exponent.
    const result = try alloc.alloc(u8, s.len + 1);
    @memcpy(result[0 .. e_idx + 1], s[0 .. e_idx + 1]);
    result[e_idx + 1] = '+';
    @memcpy(result[e_idx + 2 ..], s[e_idx + 1 ..]);
    return result;
}

/// Number.prototype.toExponential — format a number in exponential notation.
/// `fraction_digits` is optional (null = use default precision ~6 digits).
pub fn toExponential(alloc: std.mem.Allocator, val: f64, fraction_digits: ?i64) ![]const u8 {
    // Handle special values
    if (std.math.isNan(val)) return alloc.dupe(u8, "NaN");
    if (std.math.isInf(val)) {
        return if (val > 0) alloc.dupe(u8, "Infinity") else alloc.dupe(u8, "-Infinity");
    }
    // Handle -0: JS requires "0e+0" not "-0e+0"
    const safe_val: f64 = if (val == 0.0) 0.0 else val;

    // ECMA-262 §22.1.3.3: Throw RangeError if fraction_digits < 0 or > 100
    const digits: usize = if (fraction_digits) |d| blk: {
        if (d < 0 or d > 100) return error.RangeError;
        break :blk @intCast(d);
    } else 6;

    var buf: [512]u8 = undefined;

    // R8-P1-2: Zig `{e}` emits `e2` for positive exponents, but JS requires
    // `e+2`. Format into the buffer, then post-process via fixupExp.
    inline for (0..21) |p| {
        if (digits == p) {
            const fmt = comptime std.fmt.comptimePrint("{{e:.{d}}}", .{p});
            const s = std.fmt.bufPrint(&buf, fmt, .{safe_val}) catch break;
            return fixupExp(alloc, s);
        }
    }
    // Fallback for digits >= 21 (or rare buffer overflow for digits < 21):
    // Format with max comptime precision (20), then compute additional digits
    // via successive multiplication of the fractional remainder.
    // Pre-fix: padded zeros after position 20, which was wrong for values
    // like 0.1 where the exact binary representation has non-zero digits
    // beyond 15 decimal places.
    const s = std.fmt.bufPrint(&buf, "{e:.20}", .{safe_val}) catch "0.00000000000000000000e0";
    const fixed = try fixupExp(alloc, s);
    const e_pos = std.mem.indexOfScalar(u8, fixed, 'e') orelse return fixed;
    const dot_pos = std.mem.indexOfScalar(u8, fixed[0..e_pos], '.') orelse return fixed;
    const existing_frac = e_pos - dot_pos - 1;
    if (digits <= existing_frac) return fixed;
    const needed = digits - existing_frac;
    // Parse existing fractional digits after the dot as the remainder.
    const frac_str = fixed[dot_pos + 1 .. e_pos];
    const remainder: f64 = std.fmt.parseFloat(f64, frac_str) catch 0.0;
    // Compute additional digits.
    var extra_buf: [80]u8 = undefined;
    var extra_len: usize = 0;
    var rem = remainder;
    while (extra_len < needed and extra_len < extra_buf.len) {
        rem *= 10.0;
        const dgt: u8 = @intFromFloat(@floor(rem));
        extra_buf[extra_len] = '0' + @min(dgt, 9);
        extra_len += 1;
        rem -= @as(f64, @floatFromInt(dgt));
    }
    // Insert extra digits before the 'e'.
    const result = try alloc.alloc(u8, fixed.len + extra_len);
    @memcpy(result[0..e_pos], fixed[0..e_pos]);
    @memcpy(result[e_pos..e_pos + extra_len], extra_buf[0..extra_len]);
    @memcpy(result[e_pos + extra_len ..], fixed[e_pos..]);
    alloc.free(fixed);
    return result;
}

/// Number.prototype.toPrecision — format with specified significant digits.
/// `precision` is optional (null = use default 6 significant digits).
///
/// R8-P1-1: Previously this always emitted exponential notation. Per the JS
/// spec, fixed-point is used when -6 <= e < p (where e is the decimal
/// exponent of the leading significant digit and p is the precision).
pub fn toPrecision(alloc: std.mem.Allocator, val: f64, precision: ?i64) ![]const u8 {
    // Handle special values
    if (std.math.isNan(val)) return alloc.dupe(u8, "NaN");
    if (std.math.isInf(val)) {
        return if (val > 0) alloc.dupe(u8, "Infinity") else alloc.dupe(u8, "-Infinity");
    }

    // ECMA-262 §22.1.3.5: Throw RangeError if precision < 1 or > 100
    const p: usize = if (precision) |d| blk: {
        if (d < 1 or d > 100) return error.RangeError;
        break :blk @intCast(d);
    } else 6;

    if (val == 0.0) {
        // Zero: "0" for p=1, else "0." + (p-1) zeros. No sign for ±0.
        if (p == 1) return alloc.dupe(u8, "0");
        const result = try alloc.alloc(u8, 1 + 1 + (p - 1)); // "0" + "." + zeros
        result[0] = '0';
        result[1] = '.';
        for (0..(p - 1)) |i| {
            result[2 + i] = '0';
        }
        return result;
    }

    const negative = val < 0;
    const abs_val: f64 = if (negative) -val else val;
    const prefix: []const u8 = if (negative) "-" else "";

    // Format in exponential notation with (p-1) digits after the decimal,
    // yielding exactly p significant digits.
    var buf: [512]u8 = undefined;
    const exp_s: []const u8 = blk: {
        inline for (0..21) |prec| {
            if (p - 1 == prec) {
                const fmt = comptime std.fmt.comptimePrint("{{e:.{d}}}", .{prec});
                const s = std.fmt.bufPrint(&buf, fmt, .{abs_val}) catch break;
                break :blk s;
            }
        }
        // Use max comptime precision (20) for the fallback. The code below
        // extracts exactly p significant digits from the mantissa, padding
        // with zeros if p > 21 (f64 precision is ~15.95 decimal digits).
        break :blk std.fmt.bufPrint(&buf, "{e:.20}", .{abs_val}) catch "0.00000000000000000000e0";
    };

    // Locate 'e' to split mantissa and exponent.
    const e_idx = std.mem.indexOfScalar(u8, exp_s, 'e') orelse
        std.mem.indexOfScalar(u8, exp_s, 'E') orelse {
            return std.fmt.allocPrint(alloc, "{s}{s}", .{ prefix, exp_s });
        };
    const mantissa_str = exp_s[0..e_idx];
    const exp_str = exp_s[e_idx + 1 ..];
    const exponent = std.fmt.parseInt(i32, exp_str, 10) catch {
        return fixupExp(alloc, exp_s);
    };

    // Build the digit string (mantissa without the decimal point) and ensure
    // exactly p digits so fixed-point construction is well-defined.
    var digits_buf: [128]u8 = undefined;
    var digits_len: usize = 0;
    for (mantissa_str) |c| {
        if (c != '.') {
            digits_buf[digits_len] = c;
            digits_len += 1;
        }
    }
    // R16: When p > digits_len (p > 21), compute additional significant digits
    // via successive multiplication instead of padding with zeros.
    // Pre-fix: "0" padding made (0.1).toPrecision(30) produce "0." + 28 zeros
    // instead of "0.1000000000000000055511151231".
    if (digits_len < p and digits_len > 0) {
        // Parse the digits we have so far as f64, compute the remainder,
        // then extract additional digits.
        // The mantissa from {e:.20} gives us 21 significant digits.
        // We reconstruct the fractional part of the normalized mantissa.
        const dot_pos_m = std.mem.indexOfScalar(u8, mantissa_str, '.') orelse mantissa_str.len;
        const frac_str_m = if (dot_pos_m < mantissa_str.len) mantissa_str[dot_pos_m + 1 ..] else "";
        var rem: f64 = std.fmt.parseFloat(f64, frac_str_m) catch 0.0;
        var extra_buf: [80]u8 = undefined;
        var extra_len: usize = 0;
        const needed = p - digits_len;
        while (extra_len < needed and extra_len < extra_buf.len) {
            rem *= 10.0;
            const dgt: u8 = @intFromFloat(@floor(rem));
            extra_buf[extra_len] = '0' + @min(dgt, 9);
            extra_len += 1;
            rem -= @as(f64, @floatFromInt(dgt));
        }
        for (0..extra_len) |i| {
            digits_buf[digits_len + i] = extra_buf[i];
        }
        digits_len += extra_len;
    }
    if (digits_len > p) digits_len = p;
    while (digits_len < p) {
        digits_buf[digits_len] = '0';
        digits_len += 1;
    }
    const digits = digits_buf[0..digits_len];

    // Decide format. `exponent` is the scientific power (d.ddd × 10^exponent).
    // Fixed-point is used when -6 <= exponent < p.
    if (exponent >= -6 and exponent < @as(i32, @intCast(p))) {
        if (exponent < 0) {
            // "0." + (-e-1) zeros + p digits
            const num_zeros: usize = @intCast(-exponent - 1);
            const result = try alloc.alloc(u8, prefix.len + 2 + num_zeros + p);
            var idx: usize = 0;
            @memcpy(result[idx .. idx + prefix.len], prefix);
            idx += prefix.len;
            result[idx] = '0';
            idx += 1;
            result[idx] = '.';
            idx += 1;
            for (0..num_zeros) |i| result[idx + i] = '0';
            idx += num_zeros;
            @memcpy(result[idx .. idx + p], digits);
            return result;
        } else if (exponent < @as(i32, @intCast(p)) - 1) {
            // first (e+1) digits + "." + remaining (p-1-e) digits
            const before_dec: usize = @intCast(exponent + 1);
            const after_dec: usize = p - before_dec;
            const result = try alloc.alloc(u8, prefix.len + before_dec + 1 + after_dec);
            var idx: usize = 0;
            @memcpy(result[idx .. idx + prefix.len], prefix);
            idx += prefix.len;
            @memcpy(result[idx .. idx + before_dec], digits[0..before_dec]);
            idx += before_dec;
            result[idx] = '.';
            idx += 1;
            @memcpy(result[idx .. idx + after_dec], digits[before_dec..]);
            return result;
        } else {
            // e == p-1: integer form, no decimal point.
            return std.fmt.allocPrint(alloc, "{s}{s}", .{ prefix, digits });
        }
    }

    // Exponential form: d.dddd e ±exp (with '+' for non-negative exponent).
    const exp_sign: u8 = if (exponent >= 0) '+' else '-';
    const abs_exp: u32 = if (exponent >= 0) @intCast(exponent) else @intCast(-exponent);
    if (p == 1) {
        return std.fmt.allocPrint(alloc, "{s}{c}e{c}{d}", .{ prefix, digits[0], exp_sign, abs_exp });
    }
    return std.fmt.allocPrint(alloc, "{s}{c}.{s}e{c}{d}", .{ prefix, digits[0], digits[1..], exp_sign, abs_exp });
}

/// Number.prototype.toString([radix]) — convert a number to its string
/// representation in the given base. Matches ECMA-262 21.1.3.7.
///
/// `radix` is required (the emitter always supplies a default of 10 when JS
/// omits it, following the slice/substring/parseInt convention). Valid range
/// is 2..36 inclusive; out of range returns `error.RangeError`.
///
/// - NaN returns "NaN", ±Infinity return "Infinity"/"-Infinity", ±0 return "0".
/// - Radix 10 defers to Zig's default `{}` formatter (shortest round-trip
///   fixed-point), matching `JsValue.asString`'s float branch and the R8-E2
///   fix in `lower/helpers.rs`. JS-spec deviations for |x| >= 1e21 / |x| < 1e-6
///   (exponential form) are inherited from that path and tracked separately.
/// - Other radixes use successive division on the integer part and successive
///   multiplication on the fractional part, capping the fractional length at
///   52 digits (IEEE 754 double mantissa precision). Per ECMA-262 note 1,
///   fractional precision for non-decimal radixes is implementation-defined.
///
/// R8-NumberToString: Previously every `.toString()` call on a numeric
/// receiver was silently mis-routed to `js_date.toString`, producing both
/// wrong output and (for variable receivers) a Zig compile error. This
/// runtime function plus the lowerer/emitter rewrite restores JS-spec
/// semantics for radix 2..36.
pub fn toString(alloc: std.mem.Allocator, val: f64, radix: i64) ![]const u8 {
    // ECMA-262 step 4: validate radix BEFORE special-value handling so
    // e.g. `(NaN).toString(1)` throws RangeError even though
    // `(NaN).toString(10)` returns "NaN".
    if (radix < 2 or radix > 36) {
        return error.RangeError;
    }
    // Special values — same as toFixed/toExponential conventions.
    if (std.math.isNan(val)) return alloc.dupe(u8, "NaN");
    if (std.math.isInf(val)) {
        return if (val > 0) alloc.dupe(u8, "Infinity") else alloc.dupe(u8, "-Infinity");
    }
    if (val == 0.0) return alloc.dupe(u8, "0"); // ±0 → "0"

    // Negative handling: "-" prefix + recurse semantics via abs_val.
    const negative = val < 0;
    const abs_val: f64 = if (negative) -val else val;
    const prefix: []const u8 = if (negative) "-" else "";

    // Radix 10: defer to Zig's shortest-round-trip default formatter, which
    // matches JS Number.prototype.toString() for the common range.
    if (radix == 10) {
        var buf: [512]u8 = undefined;
        const s = try std.fmt.bufPrint(&buf, "{}", .{abs_val});
        if (prefix.len == 0) return alloc.dupe(u8, s);
        const result = try alloc.alloc(u8, prefix.len + s.len);
        @memcpy(result[0..prefix.len], prefix);
        @memcpy(result[prefix.len .. prefix.len + s.len], s);
        return result;
    }

    // Radix 2..36 (other than 10): successive-division algorithm.
    const base: f64 = @as(f64, @floatFromInt(radix));
    const digit_chars = "0123456789abcdefghijklmnopqrstuvwxyz";

    // Integer part built reversed, then flipped. Buffer covers
    // log_2(Number.MAX_VALUE) ≈ 1024 digits plus generous slack.
    var int_digits_buf: [1280]u8 = undefined;
    var int_digits_len: usize = 0;
    {
        var dividend: f64 = if (abs_val >= 1.0) @floor(abs_val) else 0.0;
        if (dividend == 0.0) {
            int_digits_buf[0] = '0';
            int_digits_len = 1;
        } else {
            while (dividend >= 1.0) {
                const q: f64 = @floor(dividend / base);
                const r: f64 = dividend - q * base;
                // Clamp to valid digit range to prevent @intFromFloat panic
                // from floating-point precision edge cases (P1-3).
                const r_clamped: f64 = @max(0.0, @min(r, base - 1.0));
                const digit_idx: usize = @intFromFloat(r_clamped);
                int_digits_buf[int_digits_len] = digit_chars[digit_idx];
                int_digits_len += 1;
                dividend = q;
            }
            // Reverse so the most-significant digit comes first.
            const half = int_digits_len / 2;
            for (0..half) |i| {
                const j = int_digits_len - 1 - i;
                const tmp = int_digits_buf[i];
                int_digits_buf[i] = int_digits_buf[j];
                int_digits_buf[j] = tmp;
            }
        }
    }
    const int_str: []const u8 = int_digits_buf[0..int_digits_len];

    // Fractional part: successive multiplication, capped at 52 digits.
    var frac_digits_buf: [64]u8 = undefined;
    var frac_digits_len: usize = 0;
    {
        var f: f64 = abs_val - @floor(abs_val);
        const max_frac_digits: usize = 52;
        while (f != 0.0 and frac_digits_len < max_frac_digits) {
            f *= base;
            const d: f64 = @floor(f);
            // Clamp to valid digit range (P1-3).
            const d_clamped: f64 = @max(0.0, @min(d, base - 1.0));
            const d_idx: usize = @intFromFloat(d_clamped);
            frac_digits_buf[frac_digits_len] = digit_chars[d_idx];
            frac_digits_len += 1;
            f -= d;
        }
    }

    // If no fractional part, emit prefix + integer only.
    if (frac_digits_len == 0) {
        if (prefix.len == 0) return alloc.dupe(u8, int_str);
        const result = try alloc.alloc(u8, prefix.len + int_str.len);
        @memcpy(result[0..prefix.len], prefix);
        @memcpy(result[prefix.len .. prefix.len + int_str.len], int_str);
        return result;
    }

    // Construct prefix + int_str + "." + frac_digits on the heap.
    const total_len = prefix.len + int_str.len + 1 + frac_digits_len;
    const result = try alloc.alloc(u8, total_len);
    @memcpy(result[0..prefix.len], prefix);
    @memcpy(result[prefix.len .. prefix.len + int_str.len], int_str);
    result[prefix.len + int_str.len] = '.';
    const frac_off = prefix.len + int_str.len + 1;
    @memcpy(result[frac_off .. frac_off + frac_digits_len], frac_digits_buf[0..frac_digits_len]);
    return result;
}

// ── Tests ──

test "isNaN" {
    try std.testing.expect(isNaN(std.math.nan(f64)));
    try std.testing.expect(!isNaN(42.0));
}

test "isFinite" {
    try std.testing.expect(isFinite(42.0));
    try std.testing.expect(!isFinite(std.math.inf(f64)));
    try std.testing.expect(!isFinite(std.math.nan(f64)));
}

test "isInteger" {
    try std.testing.expect(isInteger(42.0));
    try std.testing.expect(isInteger(-7.0));
    try std.testing.expect(!isInteger(3.14));
    try std.testing.expect(!isInteger(std.math.nan(f64)));
}

test "parseInt" {
    try std.testing.expectEqual(@as(i64, 42), parseInt("42", null));
    try std.testing.expectEqual(@as(i64, 0), parseInt("abc", null));
    // JS semantics: whitespace trimmed
    try std.testing.expectEqual(@as(i64, 123), parseInt("   123 ", null));
    // JS semantics: stops at decimal point
    try std.testing.expectEqual(@as(i64, 1), parseInt("1.9", null));
    // JS semantics: 0x prefix auto-detected
    try std.testing.expectEqual(@as(i64, 255), parseInt("0xFF", null));
    try std.testing.expectEqual(@as(i64, 255), parseInt("0xFF", 16));
    // JS semantics: leading zeros ignored in base 10
    try std.testing.expectEqual(@as(i64, 77), parseInt("077", null));
    // JS semantics: hex digits with explicit radix
    try std.testing.expectEqual(@as(i64, 255), parseInt("ff", 16));
}

test "parseInt does not recognize 0b/0o prefixes (P2-8)" {
    // Per ECMAScript spec, parseInt only recognizes 0x/0X prefix.
    // 0b/0B and 0o/0O are NOT recognized — "0" is parsed as digit 0,
    // then 'b'/'o' stops parsing. Only Number() recognizes these prefixes.
    try std.testing.expectEqual(@as(i64, 0), parseInt("0b1010", null));
    try std.testing.expectEqual(@as(i64, 0), parseInt("0B1010", null));
    try std.testing.expectEqual(@as(i64, 0), parseInt("0o17", null));
    try std.testing.expectEqual(@as(i64, 0), parseInt("0O17", null));
    // Even with matching radix, prefix is not stripped.
    try std.testing.expectEqual(@as(i64, 0), parseInt("0b1010", 2));
    try std.testing.expectEqual(@as(i64, 0), parseInt("0o17", 8));
    // 0x prefix still works.
    try std.testing.expectEqual(@as(i64, 255), parseInt("0xFF", null));
    try std.testing.expectEqual(@as(i64, 255), parseInt("0xFF", 16));
}

test "parseInt overflow does not panic (R6-7)" {
    // JS parseInt returns a Number (f64) — values exceeding i64 range used
    // to panic in the i64 accumulator. Now we clamp to nearest representable
    // i64 instead. Verify a few overflow cases return without panicking.
    const huge = parseInt("99999999999999999999", null);
    try std.testing.expectEqual(@as(i64, std.math.maxInt(i64)), huge);
    const huge_neg = parseInt("-99999999999999999999", null);
    try std.testing.expectEqual(@as(i64, std.math.minInt(i64)), huge_neg);
    // 2^63 magnitude exactly (= -minInt(i64)) parses to minInt(i64) (negative)
    try std.testing.expectEqual(@as(i64, std.math.minInt(i64)), parseInt("-9223372036854775808", null));
    // 2^63 (one past maxInt) positive overflows → clamp
    try std.testing.expectEqual(@as(i64, std.math.maxInt(i64)), parseInt("9223372036854775808", null));
    // Sanity: maxInt(i64) itself still parses exactly
    try std.testing.expectEqual(@as(i64, 9223372036854775807), parseInt("9223372036854775807", null));
    // Hex overflow path also must not panic
    const hex_huge = parseInt("ffffffffffffffffffffffff", 16);
    try std.testing.expectEqual(@as(i64, std.math.maxInt(i64)), hex_huge);
}

test "parseFloat" {
    try std.testing.expectEqual(@as(f64, 3.14), parseFloat("3.14"));
    try std.testing.expect(std.math.isNan(parseFloat("abc")));
}

test "isSafeInteger" {
    try std.testing.expect(isSafeInteger(42.0));
    try std.testing.expect(isSafeInteger(@as(f64, 9007199254740991)));
    try std.testing.expect(isSafeInteger(@as(f64, -9007199254740991)));
    try std.testing.expect(!isSafeInteger(@as(f64, 9007199254740992)));
    try std.testing.expect(!isSafeInteger(@as(f64, -9007199254740992)));
    try std.testing.expect(!isSafeInteger(3.14));
    try std.testing.expect(!isSafeInteger(std.math.nan(f64)));
    try std.testing.expect(!isSafeInteger(std.math.inf(f64)));
}

test "isSafeInteger out-of-i64-range does not panic (R6-8)" {
    // Finite-but-out-of-i64-range f64 values like 1e30 used to panic in
    // @intFromFloat. Now we range-check against the safe-integer bounds first.
    try std.testing.expect(!isSafeInteger(1e30));
    try std.testing.expect(!isSafeInteger(-1e30));
    try std.testing.expect(!isSafeInteger(1e300));
    try std.testing.expect(!isSafeInteger(-1e300));
    // Boundary: just outside safe range
    try std.testing.expect(!isSafeInteger(@as(f64, 9007199254740992)));
    try std.testing.expect(!isSafeInteger(@as(f64, -9007199254740992)));
    // Boundary: still inside safe range
    try std.testing.expect(isSafeInteger(@as(f64, 9007199254740991)));
    try std.testing.expect(isSafeInteger(@as(f64, -9007199254740991)));
}

test "toFixed" {
    const a = std.testing.allocator;
    const r1 = try toFixed(a, 3.14159, 2);
    defer a.free(r1);
    try std.testing.expectEqualStrings("3.14", r1);

    const r2 = try toFixed(a, 3.0, 3);
    defer a.free(r2);
    try std.testing.expectEqualStrings("3.000", r2);

    const r3 = try toFixed(a, -2.5, 0);
    defer a.free(r3);
    try std.testing.expectEqualStrings("-3", r3);

    const r4 = try toFixed(a, std.math.nan(f64), 2);
    defer a.free(r4);
    try std.testing.expectEqualStrings("NaN", r4);

    const r5 = try toFixed(a, std.math.inf(f64), 2);
    defer a.free(r5);
    try std.testing.expectEqualStrings("Infinity", r5);
}

test "toExponential" {
    const a = std.testing.allocator;
    // Test basic exponential formatting
    const r1 = try toExponential(a, 3.14159, 2);
    defer a.free(r1);
    // Should be something like "3.14e+0"
    try std.testing.expect(r1.len > 0);

    // Test with null (default precision)
    const r2 = try toExponential(a, 3.14159, null);
    defer a.free(r2);
    try std.testing.expect(r2.len > 0);

    // Test special values
    const r3 = try toExponential(a, std.math.nan(f64), 2);
    defer a.free(r3);
    try std.testing.expectEqualStrings("NaN", r3);
}

test "toPrecision" {
    const a = std.testing.allocator;
    // Test basic precision formatting
    const r1 = try toPrecision(a, 3.14159, 3);
    defer a.free(r1);
    try std.testing.expect(r1.len > 0);

    // Test with null (default precision)
    const r2 = try toPrecision(a, 3.14159, null);
    defer a.free(r2);
    try std.testing.expect(r2.len > 0);

    // Test special values
    const r3 = try toPrecision(a, std.math.nan(f64), 3);
    defer a.free(r3);
    try std.testing.expectEqualStrings("NaN", r3);
}

test "toExponential emits e+ sign for positive exponents (R8-P1-2)" {
    const a = std.testing.allocator;
    // Pre-fix: Zig {e} produced "1.2e2" without the '+'; JS requires "1.2e+2".
    {
        const r = try toExponential(a, 123.0, 1);
        defer a.free(r);
        try std.testing.expectEqualStrings("1.2e+2", r);
    }
    // Exponent 0 also needs the '+' sign.
    {
        const r = try toExponential(a, 1.0, 0);
        defer a.free(r);
        try std.testing.expectEqualStrings("1e+0", r);
    }
    // Negative exponents keep the '-' (already correct).
    {
        const r = try toExponential(a, 0.012, 1);
        defer a.free(r);
        try std.testing.expectEqualStrings("1.2e-2", r);
    }
    // Large positive exponent with explicit digits.
    {
        const r = try toExponential(a, 10000.0, 2);
        defer a.free(r);
        try std.testing.expectEqualStrings("1.00e+4", r);
    }
    // Negative value keeps the leading '-'.
    {
        const r = try toExponential(a, -123.0, 1);
        defer a.free(r);
        try std.testing.expectEqualStrings("-1.2e+2", r);
    }
}

test "toPrecision uses fixed-point when -6 <= e < p (R8-P1-1)" {
    const a = std.testing.allocator;
    // Pre-fix: always exponential. JS uses fixed-point here.
    // e == p-1 → integer form (no decimal point).
    {
        const r = try toPrecision(a, 123.0, 3);
        defer a.free(r);
        try std.testing.expectEqualStrings("123", r);
    }
    // 0 <= e < p-1 → digits before and after the decimal.
    {
        const r = try toPrecision(a, 123.0, 5);
        defer a.free(r);
        try std.testing.expectEqualStrings("123.00", r);
    }
    // e >= p → exponential.
    {
        const r = try toPrecision(a, 123.0, 2);
        defer a.free(r);
        try std.testing.expectEqualStrings("1.2e+2", r);
    }
    // e < 0 (within -6) → "0." + zeros + digits.
    {
        const r = try toPrecision(a, 0.000123, 3);
        defer a.free(r);
        try std.testing.expectEqualStrings("0.000123", r);
    }
    // e < -6 → exponential.
    {
        const r = try toPrecision(a, 0.000000123, 2);
        defer a.free(r);
        try std.testing.expectEqualStrings("1.2e-7", r);
    }
    // Zero with precision.
    {
        const r = try toPrecision(a, 0.0, 3);
        defer a.free(r);
        try std.testing.expectEqualStrings("0.00", r);
    }
    // Negative value with exponential form keeps '-'.
    {
        const r = try toPrecision(a, -123.0, 2);
        defer a.free(r);
        try std.testing.expectEqualStrings("-1.2e+2", r);
    }
    // Integer form at boundary: e == p-1.
    {
        const r = try toPrecision(a, 1234.0, 4);
        defer a.free(r);
        try std.testing.expectEqualStrings("1234", r);
    }
}

test "toString radix 10 default (R8-NumberToString)" {
    const a = std.testing.allocator;
    // Easiest case: matches JS `(42).toString()` (i.e. no radix).
    {
        const r = try toString(a, 42.0, 10);
        defer a.free(r);
        try std.testing.expectEqualStrings("42", r);
    }
    // Negative integer.
    {
        const r = try toString(a, -42.0, 10);
        defer a.free(r);
        try std.testing.expectEqualStrings("-42", r);
    }
    // Fractional in shortest round-trip form (R8-E2).
    {
        const r = try toString(a, 3.14, 10);
        defer a.free(r);
        try std.testing.expectEqualStrings("3.14", r);
    }
    // 0.1 (the round-trip-canonical value).
    {
        const r = try toString(a, 0.1, 10);
        defer a.free(r);
        try std.testing.expectEqualStrings("0.1", r);
    }
    // ±0 → "0" (ECMA-262 21.1.3.7 step 8: "-" is only emitted when x < 0).
    {
        const r = try toString(a, 0.0, 10);
        defer a.free(r);
        try std.testing.expectEqualStrings("0", r);
    }
    {
        const r = try toString(a, -0.0, 10);
        defer a.free(r);
        try std.testing.expectEqualStrings("0", r);
    }
    // Special values.
    {
        const r = try toString(a, std.math.nan(f64), 10);
        defer a.free(r);
        try std.testing.expectEqualStrings("NaN", r);
    }
    {
        const r = try toString(a, std.math.inf(f64), 10);
        defer a.free(r);
        try std.testing.expectEqualStrings("Infinity", r);
    }
    {
        const r = try toString(a, -std.math.inf(f64), 10);
        defer a.free(r);
        try std.testing.expectEqualStrings("-Infinity", r);
    }
}

test "toString non-decimal radix integers (R8-NumberToString)" {
    const a = std.testing.allocator;
    // Binary.
    {
        const r = try toString(a, 42.0, 2);
        defer a.free(r);
        try std.testing.expectEqualStrings("101010", r);
    }
    // Octal.
    {
        const r = try toString(a, 8.0, 8);
        defer a.free(r);
        try std.testing.expectEqualStrings("10", r);
    }
    {
        const r = try toString(a, 7.0, 8);
        defer a.free(r);
        try std.testing.expectEqualStrings("7", r);
    }
    // Hex.
    {
        const r = try toString(a, 255.0, 16);
        defer a.free(r);
        try std.testing.expectEqualStrings("ff", r);
    }
    {
        const r = try toString(a, 4096.0, 16);
        defer a.free(r);
        try std.testing.expectEqualStrings("1000", r);
    }
    // Base 36 wraps around to letters for digits >= 10.
    {
        const r = try toString(a, 35.0, 36);
        defer a.free(r);
        try std.testing.expectEqualStrings("z", r);
    }
    {
        const r = try toString(a, 36.0, 36);
        defer a.free(r);
        try std.testing.expectEqualStrings("10", r);
    }
    // Negative values in non-decimal radix carry the '-' prefix.
    {
        const r = try toString(a, -10.0, 2);
        defer a.free(r);
        try std.testing.expectEqualStrings("-1010", r);
    }
    {
        const r = try toString(a, -255.0, 16);
        defer a.free(r);
        try std.testing.expectEqualStrings("-ff", r);
    }
}

test "toString non-decimal radix fractional (R8-NumberToString)" {
    const a = std.testing.allocator;
    // 0.5 in binary is exactly 0.1.
    {
        const r = try toString(a, 0.5, 2);
        defer a.free(r);
        try std.testing.expectEqualStrings("0.1", r);
    }
    // 0.125 in binary is exactly 0.001.
    {
        const r = try toString(a, 0.125, 2);
        defer a.free(r);
        try std.testing.expectEqualStrings("0.001", r);
    }
    // Mixed integer and fractional parts in binary.
    {
        const r = try toString(a, 5.25, 2);
        defer a.free(r);
        try std.testing.expectEqualStrings("101.01", r);
    }
    // 0.5 in hex is exactly 0.8.
    {
        const r = try toString(a, 255.5, 16);
        defer a.free(r);
        try std.testing.expectEqualStrings("ff.8", r);
    }
}

test "toString rejects out-of-range radix (R8-NumberToString)" {
    const a = std.testing.allocator;
    // Radix 0 / 1 / 37 / -1 all must throw RangeError.
    try std.testing.expectError(error.RangeError, toString(a, 42.0, 0));
    try std.testing.expectError(error.RangeError, toString(a, 42.0, 1));
    try std.testing.expectError(error.RangeError, toString(a, 42.0, 37));
    try std.testing.expectError(error.RangeError, toString(a, 42.0, -1));
    // Radix validation must run BEFORE the special-value short-circuit:
    // `(NaN).toString(1)` throws RangeError even though
    // `(NaN).toString(10)` returns "NaN".
    try std.testing.expectError(error.RangeError, toString(a, std.math.nan(f64), 1));
    try std.testing.expectError(error.RangeError, toString(a, std.math.inf(f64), 0));
}
