// test_date_simple.js
// Simple Date tests (without new Date())

export function testDateNow() {
    const now = Date.now();
    // Should be a positive number (milliseconds since epoch)
    if (now > 0) {
        return 1;
    }
    return 0;
}

// NOTE: new Date() is not supported by the transpiler.
// To test Date instance methods (getFullYear, getMonth, etc.),
// we would need to:
//   1. Call Date.now() to get milliseconds
//   2. Pass milliseconds to js_date.getFullYear(millis)
// But in JS, we can't call .getFullYear() on a number.
//
// Workaround: The transpiler's native_proto system detects
// date.getFullYear() and generates js_date.getFullYear(date).
// If `date` holds milliseconds (i64), this works in Zig.
//
// However, the JS parser rejects millis.getFullYear().
// This is a known limitation. For proper Date testing,
// write Zig tests in runtime/js_date.zig directly.
