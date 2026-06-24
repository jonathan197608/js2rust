// phase6.js — Phase 6: Built-in Objects Deep Dive
// Tests: String, Math, Date, Number methods
// All functions use i64/bool returns for C ABI simplicity

// ════════════════════════════════════════════════════════════════
// ── Category 1: String instance methods ────────────────────────
// Using variables (not string literals) as callee objects
// because callee_object_name() only works on Identifier objects
// ════════════════════════════════════════════════════════════════

// ── String.indexOf ───────────────────────────────────
/**
 * @returns {i64}
 */
export function testStringIndexOf() {
    const s = "hello";
    if (s.indexOf("e") === 1) { return 0; }
    return -1;
}

// ── String.indexOf not found ─────────────────────────
/**
 * @returns {i64}
 */
export function testStringIndexOfNotFound() {
    const s = "hello";
    if (s.indexOf("z") === -1) { return 0; }
    return -1;
}

// ── String.includes ──────────────────────────────────
/**
 * @returns {i64}
 */
export function testStringIncludes() {
    const s = "hello world";
    if (s.includes("world")) { return 0; }
    return -1;
}

// ── String.includes not found ────────────────────────
/**
 * @returns {i64}
 */
export function testStringIncludesNotFound() {
    const s = "hello";
    if (!s.includes("xyz")) { return 0; }
    return -1;
}

// ── String.startsWith ────────────────────────────────
/**
 * @returns {i64}
 */
export function testStringStartsWith() {
    const s = "hello world";
    if (s.startsWith("hello")) { return 0; }
    return -1;
}

// ── String.startsWith false ──────────────────────────
/**
 * @returns {i64}
 */
export function testStringStartsWithFalse() {
    const s = "hello";
    if (!s.startsWith("xyz")) { return 0; }
    return -1;
}

// ── String.endsWith ──────────────────────────────────
/**
 * @returns {i64}
 */
export function testStringEndsWith() {
    const s = "hello world";
    if (s.endsWith("world")) { return 0; }
    return -1;
}

// ── String.endsWith false ────────────────────────────
/**
 * @returns {i64}
 */
export function testStringEndsWithFalse() {
    const s = "hello";
    if (!s.endsWith("abc")) { return 0; }
    return -1;
}

// ── String.trim ──────────────────────────────────────
// Verifies trim() doesn't crash; uses stored result to check
/**
 * @returns {i64}
 */
export function testStringTrim() {
    const s = "  hello  ";
    const t = s.trim();
    // trim() returns a string; verify the first char is 'h'
    if (t.startsWith("hello")) { return 0; }
    return -1;
}

// ════════════════════════════════════════════════════════════════
// ── Category 2: Math static methods ────────────────────────────
// ════════════════════════════════════════════════════════════════

// ── Math.abs ─────────────────────────────────────────
/**
 * @returns {i64}
 */
export function testMathAbs() {
    if (Math.abs(-42) === 42 && Math.abs(42) === 42) { return 0; }
    return -1;
}

// ── Math.floor ───────────────────────────────────────
/**
 * @returns {i64}
 */
export function testMathFloor() {
    if (Math.floor(3.7) === 3) { return 0; }
    return -1;
}

// ── Math.ceil ────────────────────────────────────────
/**
 * @returns {i64}
 */
export function testMathCeil() {
    if (Math.ceil(3.2) === 4) { return 0; }
    return -1;
}

// ── Math.round ───────────────────────────────────────
/**
 * @returns {i64}
 */
export function testMathRound() {
    if (Math.round(3.4) === 3 && Math.round(3.6) === 4) { return 0; }
    return -1;
}

// ── Math.max (3 args) ────────────────────────────────
/**
 * @returns {i64}
 */
export function testMathMax() {
    if (Math.max(10, 25, 5) === 25) { return 0; }
    return -1;
}

// ── Math.min (3 args) ────────────────────────────────
/**
 * @returns {i64}
 */
export function testMathMin() {
    if (Math.min(10, 25, 5) === 5) { return 0; }
    return -1;
}

// ════════════════════════════════════════════════════════════════
// ── Category 3: Date (new + instance methods) ──────────────────
// ════════════════════════════════════════════════════════════════

// ── new Date().getTime() ─────────────────────────────
/**
 * @returns {i64}
 */
export function testDateNow() {
    // new Date() creates a JsDate struct, getTime() is an instance method
    if (new Date().getTime() > 0) { return 0; }
    return -1;
}

// ── new Date(millis).getTime() ───────────────────────
/**
 * @returns {i64}
 */
export function testDateFromMillis() {
    // 1000 millis = epoch + 1 second
    if (new Date(1000).getTime() === 1000) { return 0; }
    return -1;
}

// ── Date.getFullYear ─────────────────────────────────
/**
 * @returns {i64}
 */
export function testDateGetFullYear() {
    // epoch = 1970-01-01
    if (new Date(0).getFullYear() === 1970) { return 0; }
    return -1;
}

// ── Date.getMonth ────────────────────────────────────
/**
 * @returns {i64}
 */
export function testDateGetMonth() {
    // epoch = January (0)
    if (new Date(0).getMonth() === 0) { return 0; }
    return -1;
}

// ── Date.getDate ─────────────────────────────────────
/**
 * @returns {i64}
 */
export function testDateGetDate() {
    if (new Date(0).getDate() === 1) { return 0; }
    return -1;
}

// ── Date.getDay ──────────────────────────────────────
/**
 * @returns {i64}
 */
export function testDateGetDay() {
    // epoch = Thursday (4)
    if (new Date(0).getDay() === 4) { return 0; }
    return -1;
}

// ── Date.getHours ────────────────────────────────────
/**
 * @returns {i64}
 */
export function testDateGetHours() {
    if (new Date(0).getHours() === 0) { return 0; }
    return -1;
}

// ── Date.now() static method ─────────────────────────
/**
 * @returns {i64}
 */
export function testDateNowStatic() {
    if (Date.now() > 0) { return 0; }
    return -1;
}

// ════════════════════════════════════════════════════════════════
// ── Category 4: Number (parseInt) ──────────────────────────────
// ════════════════════════════════════════════════════════════════

// ── parseInt ─────────────────────────────────────────
/**
 * @returns {i64}
 */
export function testParseInt() {
    if (parseInt("42") === 42) { return 0; }
    return -1;
}

// ── parseInt hex ─────────────────────────────────────
/**
 * @returns {i64}
 */
export function testParseIntHex() {
    if (parseInt("FF", 16) === 255) { return 0; }
    return -1;
}

// ════════════════════════════════════════════════════════════════
// ── Category 5: Object static methods ──────────────────────────
// ════════════════════════════════════════════════════════════════

// ── Object.keys ──────────────────────────────────────
/**
 * @returns {i64}
 */
export function testObjectKeys() {
    const obj = { a: 1, b: 2, c: 3 };
    if (Object.keys(obj).length === 3) { return 0; }
    return -1;
}
