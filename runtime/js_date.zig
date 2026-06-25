//! JS Date method implementations for Zig.
//! Uses Howard Hinnant's civil calendar algorithms for accurate date math.

const std = @import("std");
const builtin = @import("builtin");

/// JsDate struct — represents a Date object in generated Zig code.
/// Stores milliseconds since epoch as i64.
pub const JsDate = struct {
    millis: i64,

    /// new Date() — current time
    pub fn init() JsDate {
        return .{ .millis = milliTimestamp() };
    }

    /// new Date(millis) — from timestamp
    pub fn fromMillis(millis: i64) JsDate {
        return .{ .millis = millis };
    }

    // ── Local-time getters ──

    pub fn getTime(self: JsDate) i64 {
        return self.millis;
    }

    pub fn getFullYear(self: JsDate) i64 {
        const cd = civilFromDays(dayCount(self.millis));
        return cd.y;
    }

    pub fn getMonth(self: JsDate) i64 {
        const cd = civilFromDays(dayCount(self.millis));
        return cd.m - 1; // 0-indexed
    }

    pub fn getDate(self: JsDate) i64 {
        const cd = civilFromDays(dayCount(self.millis));
        return cd.d;
    }

    pub fn getDay(self: JsDate) i64 {
        const days = dayCount(self.millis);
        // 1970-01-01 = Thursday = 4 (0=Sun)
        return @mod(days + 4, 7);
    }

    pub fn getHours(self: JsDate) i64 {
        return timePart(self.millis, 3600 * 1000, 24);
    }

    pub fn getMinutes(self: JsDate) i64 {
        return timePart(self.millis, 60 * 1000, 60);
    }

    pub fn getSeconds(self: JsDate) i64 {
        return timePart(self.millis, 1000, 60);
    }

    pub fn getMilliseconds(self: JsDate) i64 {
        return @mod(self.millis, 1000);
    }

    pub fn getTimezoneOffset(self: JsDate) i64 {
        _ = self;
        return 0; // UTC only for now
    }

    /// Returns ISO 8601 string: "YYYY-MM-DDTHH:mm:ss.sssZ"
    pub fn toISOString(self: JsDate, alloc: std.mem.Allocator) ![]const u8 {
        const cd = civilFromDays(dayCount(self.millis));
        const h = timePart(self.millis, 3600 * 1000, 24);
        const min = timePart(self.millis, 60 * 1000, 60);
        const s = timePart(self.millis, 1000, 60);
        const ms = @mod(self.millis, 1000);
        // Use unsigned casts to avoid {d} sign prefix for positive numbers
        return std.fmt.allocPrint(alloc, "{d:0>4}-{d:0>2}-{d:0>2}T{d:0>2}:{d:0>2}:{d:0>2}.{d:0>3}Z", .{
            @as(u64, @intCast(cd.y)),
            @as(u64, @intCast(cd.m)),
            @as(u64, @intCast(cd.d)),
            @as(u64, @intCast(h)),
            @as(u64, @intCast(min)),
            @as(u64, @intCast(s)),
            @as(u64, @intCast(@abs(ms))),
        });
    }

    /// Returns RFC 2822 format: "Wed Apr 12 2023 12:00:00 GMT"
    pub fn toString(self: JsDate, alloc: std.mem.Allocator) ![]const u8 {
        const cd = civilFromDays(dayCount(self.millis));
        const h = timePart(self.millis, 3600 * 1000, 24);
        const min = timePart(self.millis, 60 * 1000, 60);
        const s = timePart(self.millis, 1000, 60);

        const day_names = [_][]const u8{ "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat" };
        const month_names = [_][]const u8{ "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec" };

        const dow = @mod(dayCount(self.millis) + 4, 7); // 0=Sun

        return std.fmt.allocPrint(alloc, "{s} {s} {d} {d} {d}:{d}:{d} GMT", .{
            day_names[@intCast(dow)],
            month_names[@intCast(cd.m - 1)],
            @as(u64, @intCast(cd.d)),
            @as(u64, @intCast(cd.y)),
            @as(u64, @intCast(h)),
            @as(u64, @intCast(min)),
            @as(u64, @intCast(s)),
        });
    }

    /// Returns date portion only: "Wed Apr 12 2023"
    pub fn toDateString(self: JsDate, alloc: std.mem.Allocator) ![]const u8 {
        const cd = civilFromDays(dayCount(self.millis));

        const day_names = [_][]const u8{ "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat" };
        const month_names = [_][]const u8{ "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec" };

        const dow = @mod(dayCount(self.millis) + 4, 7); // 0=Sun

        return std.fmt.allocPrint(alloc, "{s} {s} {d} {d}", .{
            day_names[@intCast(dow)],
            month_names[@intCast(cd.m - 1)],
            @as(u64, @intCast(cd.d)),
            @as(u64, @intCast(cd.y)),
        });
    }

    /// Returns time portion only: "12:00:00 GMT"
    pub fn toTimeString(self: JsDate, alloc: std.mem.Allocator) ![]const u8 {
        const h = timePart(self.millis, 3600 * 1000, 24);
        const min = timePart(self.millis, 60 * 1000, 60);
        const s = timePart(self.millis, 1000, 60);

        return std.fmt.allocPrint(alloc, "{d:0>2}:{d:0>2}:{d:0>2} GMT", .{
            @as(u64, @intCast(h)),
            @as(u64, @intCast(min)),
            @as(u64, @intCast(s)),
        });
    }

    /// Locale-specific string (simplified: same as toString())
    pub fn toLocaleString(self: JsDate, alloc: std.mem.Allocator) ![]const u8 {
        return self.toString(alloc);
    }

    // ── UTC getters (same as local-time for UTC-only implementation) ──

    pub fn getUTCFullYear(self: JsDate) i64 {
        return self.getFullYear();
    }

    pub fn getUTCMonth(self: JsDate) i64 {
        return self.getMonth();
    }

    pub fn getUTCDate(self: JsDate) i64 {
        return self.getDate();
    }

    pub fn getUTCDay(self: JsDate) i64 {
        return self.getDay();
    }

    pub fn getUTCHours(self: JsDate) i64 {
        return self.getHours();
    }

    pub fn getUTCMinutes(self: JsDate) i64 {
        return self.getMinutes();
    }

    pub fn getUTCSeconds(self: JsDate) i64 {
        return self.getSeconds();
    }

    pub fn getUTCMilliseconds(self: JsDate) i64 {
        return self.getMilliseconds();
    }
};

// ── Standalone helper functions (used by generated code calling Date.now() directly) ──

/// Date.now — returns current timestamp in milliseconds (i64).
pub fn now() i64 {
    return milliTimestamp();
}

/// Get milliseconds from an epoch-millis value (identity).
pub fn getTime(millis: i64) i64 {
    return millis;
}

/// GetFullYear from raw millis.
pub fn calcFullYear(millis: i64) i64 {
    const cd = civilFromDays(dayCount(millis));
    return cd.y;
}

/// Get month (0-11) from raw millis.
pub fn calcMonth(millis: i64) i64 {
    const cd = civilFromDays(dayCount(millis));
    return cd.m - 1;
}

/// Get day of month (1-31) from raw millis.
pub fn calcDate(millis: i64) i64 {
    const cd = civilFromDays(dayCount(millis));
    return cd.d;
}

/// Get day of week (0=Sun .. 6=Sat). 1970-01-01 was a Thursday.
pub fn calcDay(millis: i64) i64 {
    const days = dayCount(millis);
    return @mod(days + 4, 7);
}

/// Get hours (0-23) from raw millis.
pub fn calcHours(millis: i64) i64 {
    return timePart(millis, 3600 * 1000, 24);
}

/// Get minutes (0-59) from raw millis.
pub fn calcMinutes(millis: i64) i64 {
    return timePart(millis, 60 * 1000, 60);
}

/// Get seconds (0-59) from raw millis.
pub fn calcSeconds(millis: i64) i64 {
    return timePart(millis, 1000, 60);
}

// ── Cross-platform timestamp ──

pub fn milliTimestamp() i64 {
    return switch (builtin.os.tag) {
        .windows => milliTimestampWindows(),
        else => milliTimestampPosix(),
    };
}

fn milliTimestampWindows() i64 {
    const kernel32 = struct {
        extern "kernel32" fn GetSystemTimeAsFileTime(
            lpSystemTimeAsFileTime: *i64,
        ) callconv(.winapi) void;
    };
    var ft: i64 = undefined;
    kernel32.GetSystemTimeAsFileTime(&ft);
    const hns: u64 = @bitCast(ft);
    return @as(i64, @intCast(hns / 10000)) - 11644473600000;
}

fn milliTimestampPosix() i64 {
    var ts: std.posix.timespec = undefined;
    std.posix.system.clock_gettime(.REALTIME, &ts) catch return 0;
    return @as(i64, ts.tv_sec) * 1000 + @divTrunc(@as(i64, ts.tv_nsec), 1_000_000);
}

// ── Date math: millis ↔ civil date ──

fn dayCount(millis: i64) i64 {
    return @divFloor(millis, 86400 * 1000);
}

fn timePart(millis: i64, divisor: i64, modulus: i64) i64 {
    const t = @divFloor(@mod(millis, 86400 * 1000), divisor);
    return @mod(t, modulus);
}

/// (y, m, d) → days since 1970-01-01. Howard Hinnant's algorithm.
fn daysFromCivil(y: i64, m: i64, d: i64) i64 {
    var year = y;
    year -= @intFromBool(m <= 2);
    const era = @divFloor(if (year >= 0) year else year - 399, 400);
    const yoe: i64 = year - era * 400;
    const doy = @divFloor((153 * (if (m > 2) m - 3 else m + 9) + 2), 5) + d - 1;
    const doe = yoe * 365 + @divFloor(yoe, 4) - @divFloor(yoe, 100) + doy;
    return era * 146097 + doe - 719468;
}

/// Days since 1970-01-01 → (y, m, d). Reverse of daysFromCivil.
fn civilFromDays(days: i64) struct { y: i64, m: i64, d: i64 } {
    const z = days + 719468;
    const era = @divFloor(if (z >= 0) z else z - 146096, 146097);
    const doe = z - era * 146097; // [0, 146096]
    const yoe = @divFloor(doe - @divFloor(doe, 1460) + @divFloor(doe, 36524) - @divFloor(doe, 146096), 365);
    const y = yoe + era * 400;
    const doy = doe - (365 * yoe + @divFloor(yoe, 4) - @divFloor(yoe, 100));
    const mp = @divFloor(5 * doy + 2, 153);
    const d = doy - @divFloor(153 * mp + 2, 5) + 1;
    const m = if (mp < 10) mp + 3 else mp - 9;
    const year = y + @intFromBool(m <= 2);
    return .{ .y = year, .m = m, .d = d };
}

// ── ISO 8601 parsing ──

/// parse(dateString) — parse an ISO 8601 date string to milliseconds.
/// Supports: "YYYY-MM-DD", "YYYY-MM-DDTHH:mm:ss", "YYYY-MM-DDTHH:mm:ss.sss"
pub fn parse(s: []const u8) i64 {
    if (s.len < 10) return 0;

    const year = parseDigits4(s[0..4]) orelse return 0;
    if (s[4] != '-') return 0;
    const month = parseDigits2(s[5..7]) orelse return 0;
    if (s[7] != '-') return 0;
    const day = parseDigits2(s[8..10]) orelse return 0;

    var hours: i64 = 0;
    var minutes: i64 = 0;
    var seconds: i64 = 0;
    var millis: i64 = 0;

    if (s.len >= 19 and s[10] == 'T') {
        hours = parseDigits2(s[11..13]) orelse return 0;
        if (s[13] != ':') return 0;
        minutes = parseDigits2(s[14..16]) orelse return 0;
        if (s.len >= 19 and s[16] == ':') {
            seconds = parseDigits2(s[17..19]) orelse return 0;
        }
        if (s.len >= 21 and s[19] == '.') {
            var frac: i64 = 0;
            var mult: i64 = 100;
            var i: usize = 20;
            while (i < s.len and i < 23 and s[i] >= '0' and s[i] <= '9') : (i += 1) {
                const d = s[i] - '0';
                frac += @as(i64, d) * mult;
                mult = @divTrunc(mult, 10);
            }
            millis = frac;
        }
    }

    const days = daysFromCivil(year, month, day);
    return days * 86400 * 1000 + hours * 3600 * 1000 + minutes * 60 * 1000 + seconds * 1000 + millis;
}

fn parseDigits4(s: []const u8) ?i64 {
    if (s.len < 4) return null;
    const d0 = digit(s[0]) orelse return null;
    const d1 = digit(s[1]) orelse return null;
    const d2 = digit(s[2]) orelse return null;
    const d3 = digit(s[3]) orelse return null;
    return d0 * 1000 + d1 * 100 + d2 * 10 + d3;
}

fn parseDigits2(s: []const u8) ?i64 {
    if (s.len < 2) return null;
    const d0 = digit(s[0]) orelse return null;
    const d1 = digit(s[1]) orelse return null;
    return d0 * 10 + d1;
}

fn digit(c: u8) ?i64 {
    if (c >= '0' and c <= '9') return @as(i64, c - '0');
    return null;
}

// ── Tests ──

test "now" {
    const t = now();
    try std.testing.expect(t > 0);
}

test "getTime" {
    try std.testing.expectEqual(@as(i64, 1000), getTime(1000));
}

test "JsDate.init" {
    const d = JsDate.init();
    try std.testing.expect(d.getTime() > 0);
}

test "JsDate.fromMillis" {
    const d = JsDate.fromMillis(1000);
    try std.testing.expectEqual(@as(i64, 1000), d.getTime());
}

test "daysFromCivil epoch" {
    try std.testing.expectEqual(@as(i64, 0), daysFromCivil(1970, 1, 1));
}

test "daysFromCivil known dates" {
    try std.testing.expectEqual(@as(i64, 0), daysFromCivil(1970, 1, 1));
    try std.testing.expectEqual(@as(i64, 1), daysFromCivil(1970, 1, 2));
    try std.testing.expectEqual(@as(i64, 365), daysFromCivil(1971, 1, 1));
    try std.testing.expectEqual(@as(i64, 730), daysFromCivil(1972, 1, 1));
    try std.testing.expectEqual(@as(i64, 1096), daysFromCivil(1973, 1, 1));
}

test "civilFromDays round-trip" {
    // Test that daysFromCivil → civilFromDays round-trips for various dates
    const dates = [_]struct { y: i64, m: i64, d: i64 }{
        .{ .y = 1970, .m = 1, .d = 1 },
        .{ .y = 1970, .m = 1, .d = 2 },
        .{ .y = 1970, .m = 12, .d = 31 },
        .{ .y = 1971, .m = 1, .d = 1 },
        .{ .y = 1972, .m = 2, .d = 29 }, // leap year
        .{ .y = 2000, .m = 1, .d = 1 },
        .{ .y = 2000, .m = 12, .d = 31 },
        .{ .y = 2024, .m = 1, .d = 15 },
        .{ .y = 2024, .m = 2, .d = 29 }, // leap year
        .{ .y = 2024, .m = 12, .d = 31 },
        .{ .y = 2025, .m = 6, .d = 24 },
    };
    for (dates) |dt| {
        const days = daysFromCivil(dt.y, dt.m, dt.d);
        const cd = civilFromDays(days);
        try std.testing.expectEqual(dt.y, cd.y);
        try std.testing.expectEqual(dt.m, cd.m);
        try std.testing.expectEqual(dt.d, cd.d);
    }
}

test "JsDate getFullYear getMonth getDate" {
    // 2024-06-15T12:30:45.500
    const days = daysFromCivil(2024, 6, 15);
    const millis = (days * 86400 + 12 * 3600 + 30 * 60 + 45) * 1000 + 500;
    const d = JsDate.fromMillis(millis);
    try std.testing.expectEqual(@as(i64, 2024), d.getFullYear());
    try std.testing.expectEqual(@as(i64, 5), d.getMonth()); // June = 5 (0-indexed)
    try std.testing.expectEqual(@as(i64, 15), d.getDate());
    try std.testing.expectEqual(@as(i64, 12), d.getHours());
    try std.testing.expectEqual(@as(i64, 30), d.getMinutes());
    try std.testing.expectEqual(@as(i64, 45), d.getSeconds());
    try std.testing.expectEqual(@as(i64, 500), d.getMilliseconds());
}

test "JsDate getDay" {
    // 1970-01-01 = Thursday = 4
    try std.testing.expectEqual(@as(i64, 4), JsDate.fromMillis(0).getDay());
    // 1970-01-02 = Friday = 5
    try std.testing.expectEqual(@as(i64, 5), JsDate.fromMillis(86400 * 1000).getDay());
    // 1970-01-04 = Sunday = 0
    try std.testing.expectEqual(@as(i64, 0), JsDate.fromMillis(3 * 86400 * 1000).getDay());
}

test "JsDate getTimezoneOffset" {
    try std.testing.expectEqual(@as(i64, 0), JsDate.init().getTimezoneOffset());
}

test "JsDate toISOString" {
    const d = JsDate.fromMillis(0);
    const s = try d.toISOString(std.testing.allocator);
    defer std.testing.allocator.free(s);
    try std.testing.expectEqualStrings("1970-01-01T00:00:00.000Z", s);
}

test "JsDate toISOString non-epoch" {
    const d = JsDate.fromMillis(3723123);
    const s = try d.toISOString(std.testing.allocator);
    defer std.testing.allocator.free(s);
    try std.testing.expectEqualStrings("1970-01-01T01:02:03.123Z", s);
}

test "JsDate UTC getters" {
    const days = daysFromCivil(2024, 6, 15);
    const millis = (days * 86400 + 12 * 3600 + 30 * 60 + 45) * 1000 + 500;
    const d = JsDate.fromMillis(millis);
    try std.testing.expectEqual(@as(i64, 2024), d.getUTCFullYear());
    try std.testing.expectEqual(@as(i64, 5), d.getUTCMonth());
    try std.testing.expectEqual(@as(i64, 15), d.getUTCDate());
    try std.testing.expectEqual(@as(i64, 12), d.getUTCHours());
    try std.testing.expectEqual(@as(i64, 30), d.getUTCMinutes());
    try std.testing.expectEqual(@as(i64, 45), d.getUTCSeconds());
    try std.testing.expectEqual(@as(i64, 500), d.getUTCMilliseconds());
}

test "parse ISO 8601 date" {
    try std.testing.expectEqual(@as(i64, 0), parse("1970-01-01"));
    const d2024 = parse("2024-01-15");
    const expected2024 = (daysFromCivil(2024, 1, 15)) * 86400 * 1000;
    try std.testing.expectEqual(expected2024, d2024);
}

test "parse ISO 8601 datetime" {
    const t = parse("1970-01-01T01:02:03");
    try std.testing.expectEqual(@as(i64, 3723000), t);
}

test "parse ISO 8601 with milliseconds" {
    const t = parse("1970-01-01T00:00:00.123");
    try std.testing.expectEqual(@as(i64, 123), t);
}

test "parse invalid string" {
    try std.testing.expectEqual(@as(i64, 0), parse("not a date"));
    try std.testing.expectEqual(@as(i64, 0), parse(""));
    try std.testing.expectEqual(@as(i64, 0), parse("2024"));
}
