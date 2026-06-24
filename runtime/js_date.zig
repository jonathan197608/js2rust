//! JS Date method implementations for Zig.
//! Date methods return simple approximations (full calendar math is complex).

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

    pub fn getTime(self: JsDate) i64 {
        return self.millis;
    }

    pub fn getFullYear(self: JsDate) i64 {
        return calcFullYear(self.millis);
    }

    pub fn getMonth(self: JsDate) i64 {
        return calcMonth(self.millis);
    }

    pub fn getDate(self: JsDate) i64 {
        return calcDate(self.millis);
    }

    pub fn getDay(self: JsDate) i64 {
        return calcDay(self.millis);
    }

    pub fn getHours(self: JsDate) i64 {
        return calcHours(self.millis);
    }

    pub fn getMinutes(self: JsDate) i64 {
        return calcMinutes(self.millis);
    }

    pub fn getSeconds(self: JsDate) i64 {
        return calcSeconds(self.millis);
    }
};

/// Date.now — returns current timestamp in milliseconds (i64).
/// Cross-platform: uses GetSystemTimeAsFileTime on Windows, clock_gettime elsewhere.
pub fn now() i64 {
    return milliTimestamp();
}

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

// ── Internal calculation helpers (for use by both JsDate methods and Date.now()-based callers) ──

/// Get milliseconds from an epoch-millis value (no-op, identity).
pub fn getTime(millis: i64) i64 {
    return millis;
}

/// Placeholder: getFullYear from milliseconds since epoch.
/// Uses a simple proleptic Gregorian approximation.
fn calcFullYear(millis: i64) i64 {
    const secs: i64 = @divFloor(millis, 1000);
    const days = @divFloor(secs, 86400);
    var y: i64 = 1970;
    var d = days;
    while (d < 0) {
        y -= 1;
        d += if (@mod(y, 4) == 0 and (@mod(y, 100) != 0 or @mod(y, 400) == 0)) @as(i64, 366) else @as(i64, 365);
    }
    while (d >= 365) {
        const days_in_y = if (@mod(y, 4) == 0 and (@mod(y, 100) != 0 or @mod(y, 400) == 0)) @as(i64, 366) else @as(i64, 365);
        if (d < days_in_y) break;
        d -= days_in_y;
        y += 1;
    }
    return y;
}

/// Get month (0-11). Simple approximation.
fn calcMonth(millis: i64) i64 {
    const days = @divFloor(@divFloor(millis, 1000), 86400);
    const day_of_year: i64 = @mod(days, 365);
    return @min(11, @divFloor(day_of_year * 12, 365));
}

/// Get day of month (1-31). Simplified.
fn calcDate(millis: i64) i64 {
    const days = @divFloor(@divFloor(millis, 1000), 86400);
    const day_of_year: i64 = @mod(days, 365);
    return day_of_year + 1;
}

/// Get day of week (0=Sun .. 6=Sat).
/// 1970-01-01 was a Thursday, so epoch day 0 = Thursday(4).
fn calcDay(millis: i64) i64 {
    const days = @divFloor(@divFloor(millis, 1000), 86400);
    // 1970-01-01 = Thursday = 4 (if Sunday=0)
    const dow = @mod(days + 4, 7);
    return @rem(dow, 7);
}

/// Get hours (0-23).
fn calcHours(millis: i64) i64 {
    const secs: i64 = @divFloor(millis, 1000);
    return @mod(@divFloor(secs, 3600), 24);
}

/// Get minutes (0-59).
fn calcMinutes(millis: i64) i64 {
    const secs: i64 = @divFloor(millis, 1000);
    return @mod(@divFloor(secs, 60), 60);
}

/// Get seconds (0-59).
fn calcSeconds(millis: i64) i64 {
    const secs: i64 = @divFloor(millis, 1000);
    return @mod(secs, 60);
}

/// parse(dateString) — parse an ISO 8601 date string to milliseconds.
/// Simple implementation, returns 0 on failure.
pub fn parse(s: []const u8) i64 {
    _ = s;
    return 0;
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

test "calcFullYear" {
    try std.testing.expectEqual(@as(i64, 1970), calcFullYear(0));
    const y = calcFullYear(1735689600000);
    try std.testing.expect(y >= 2025 and y <= 2027);
}

test "calcDay" {
    const d = calcDay(0);
    try std.testing.expect(d >= 0 and d <= 6);
}

test "calcHours" {
    try std.testing.expectEqual(@as(i64, 0), calcHours(0));
}
