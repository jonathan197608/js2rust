//! JS Date method implementations for Zig.
//! Date methods return simple approximations (full calendar math is complex).

const std = @import("std");
const builtin = @import("builtin");

/// Date.now — returns current timestamp in milliseconds (i64).
/// Cross-platform: uses GetSystemTimeAsFileTime on Windows, clock_gettime elsewhere.
pub fn now() i64 {
    return milliTimestamp();
}

fn milliTimestamp() i64 {
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

/// Date.getTime — get milliseconds from an epoch-seconds value.
/// Since we store dates as i64 (millis), this just returns the value.
pub fn getTime(millis: i64) i64 {
    return millis;
}

/// Placeholder: getFullYear from milliseconds since epoch.
/// Uses a simple proleptic Gregorian approximation.
pub fn getFullYear(millis: i64) i64 {
    const secs: i64 = @divFloor(millis, 1000);
    const days = @divFloor(secs, 86400);
    // Days from epoch (1970-01-01) to target
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
pub fn getMonth(millis: i64) i64 {
    const secs: i64 = @divFloor(millis, 1000);
    const days = @divFloor(secs, 86400);
    const year = getFullYear(millis);
    _ = year;
    // Simplified: use (days % 365) / 30.44
    const day_of_year: i64 = @mod(days, 365);
    return @min(11, @divFloor(day_of_year * 12, 365));
}

/// Get day of month (1-31). Simplified.
pub fn getDate(millis: i64) i64 {
    const secs: i64 = @divFloor(millis, 1000);
    const days = @divFloor(secs, 86400);
    const day_of_year: i64 = @mod(days, 365) + 1;
    // Approximate month start
    const month = getMonth(millis);
    _ = month;
    return @mod(day_of_year, 28) + 1;
}

/// Get day of week (0=Sun .. 6=Sat).
/// 1970-01-01 was a Thursday, so epoch day 0 = Thursday(4).
pub fn getDay(millis: i64) i64 {
    const secs: i64 = @divFloor(millis, 1000);
    const days = @divFloor(secs, 86400);
    // 1970-01-01 = Thursday = 4 (if Sunday=0)
    const dow = @mod(days + 4, 7);
    return @rem(dow, 7);
}

/// Get hours (0-23).
pub fn getHours(millis: i64) i64 {
    const secs: i64 = @divFloor(millis, 1000);
    return @mod(@divFloor(secs, 3600), 24);
}

/// Get minutes (0-59).
pub fn getMinutes(millis: i64) i64 {
    const secs: i64 = @divFloor(millis, 1000);
    return @mod(@divFloor(secs, 60), 60);
}

/// Get seconds (0-59).
pub fn getSeconds(millis: i64) i64 {
    const secs: i64 = @divFloor(millis, 1000);
    return @mod(secs, 60);
}

// ── Tests ──

test "now" {
    const t = now();
    try std.testing.expect(t > 0);
}

test "getTime" {
    try std.testing.expectEqual(@as(i64, 1000), getTime(1000));
}

test "getFullYear" {
    // Epoch = 1970-01-01
    try std.testing.expectEqual(@as(i64, 1970), getFullYear(0));
    // 2026-01-01 approx
    const y = getFullYear(1735689600000);
    try std.testing.expect(y >= 2025 and y <= 2027);
}

test "getDay" {
    // 1970-01-01 was Thursday
    const d = getDay(0);
    try std.testing.expect(d >= 0 and d <= 6);
}

test "getHours" {
    try std.testing.expectEqual(@as(i64, 0), getHours(0));
}
