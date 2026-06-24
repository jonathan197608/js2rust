// test_date_timezone.js
// Test Date timezone behavior
//
// JS Date methods return values in local timezone (or UTC for getUTC* methods).
// Current implementation treats millis as UTC (no timezone adjustment).

export function testDateNow() {
    const now = Date.now();
    // Should be a positive number (milliseconds since epoch)
    return now > 0 ? 1 : 0;
}

export function testDateGetFullYear() {
    const date = new Date();
    const year = date.getFullYear();
    // Should be 2026 (or close to it)
    return (year >= 2025 && year <= 2027) ? 1 : 0;
}

export function testDateGetMonth() {
    const date = new Date();
    const month = date.getMonth();
    // Should be 0-11
    return (month >= 0 && month <= 11) ? 1 : 0;
}

export function testDateGetDate() {
    const date = new Date();
    const day = date.getDate();
    // Should be 1-31
    return (day >= 1 && day <= 31) ? 1 : 0;
}

export function testDateGetDay() {
    const date = new Date();
    const dow = date.getDay();
    // Should be 0-6 (Sunday-Saturday)
    return (dow >= 0 && dow <= 6) ? 1 : 0;
}

export function testDateGetHours() {
    const date = new Date();
    const hours = date.getHours();
    // Should be 0-23
    return (hours >= 0 && hours <= 23) ? 1 : 0;
}

export function testDateGetMinutes() {
    const date = new Date();
    const minutes = date.getMinutes();
    // Should be 0-59
    return (minutes >= 0 && minutes <= 59) ? 1 : 0;
}

export function testDateGetSeconds() {
    const date = new Date();
    const seconds = date.getSeconds();
    // Should be 0-59
    return (seconds >= 0 && seconds <= 59) ? 1 : 0;
}
