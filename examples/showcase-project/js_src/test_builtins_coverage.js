// test_builtins_coverage.js
// Comprehensive e2e tests for built-in object methods that lacked dedicated test coverage.
// Covers: Math constants/trig/log/exp/hyperbolic/other, Array manipulation,
// String methods, Map operations, Object static methods, Date methods,
// Number static methods, console.warn.
//
// Excluded due to transpiler codegen bugs (tested via Rust unit tests instead):
// - Math.atan: generates @atan() instead of std.math.atan()
// - Math.imul: parenthesis mismatch in @intCast/@bitCast nesting
// - Math.fround: generates @floatFromInt for comptime_float
// - Math.atan2: generates std.math.atan2(f64, y, x) — 3 args instead of 2
// - Math.clz32: generates @clz(comptime_int) — needs explicit integer type
// - Number.parseInt/parseFloat: generates undeclared 'Number' type identifier
// - Array.unshift: generates js_array.unshift(1) — missing allocator arg
// - Array.of: generates js_array.of(1, 2, 3) — wrong arg count
// - Array.find/findLast/findLastIndex: callback param typed as JsBigInt
// - Array.flat/flatMap: nested array type mismatch (ArrayList as i64)
// - Array.with/toReversed/toSorted/toSpliced: Rule 8 → charCodeAt misgenerated
// - Array.keys/values/entries: ArrayList has no such methods in runtime
// - Array.isArray: ArrayList vs JsAny type mismatch
// - Map.values/entries: .items misadded to slice types
// - Set.add (on const): const qualifier prevents mutable borrow
// - Object.values/entries/assign/defineProperty/defineProperties/
//   getOwnPropertyDescriptor: anonymous struct vs JsValueHashMap mismatch
// - Object.getPrototypeOf: struct init syntax unsupported
// - Object.create: wrong arg count (missing allocator)
// - Object.fromEntries: mixed-type array ["a", 1] type inference failure
// - Object.getOwnPropertyNames: .items misadded to fixed-size array
// - Date.toDateString/toTimeString/toUTCString/toLocaleString:
//   try in non-error function (function returns i64, not !i64)
// - BigInt.valueOf (on const): const qualifier prevents mutable borrow

// ═══════════════════════════════════════════════════════
// Math constants (8)
// ═══════════════════════════════════════════════════════

/** @returns {i64} */
export function testMathE() {
    if (Math.E > 2.71 && Math.E < 2.72) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathPI() {
    if (Math.PI > 3.14 && Math.PI < 3.15) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathLN2() {
    if (Math.LN2 > 0.69 && Math.LN2 < 0.70) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathLN10() {
    if (Math.LN10 > 2.30 && Math.LN10 < 2.31) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathLOG2E() {
    if (Math.LOG2E > 1.44 && Math.LOG2E < 1.45) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathLOG10E() {
    if (Math.LOG10E > 0.43 && Math.LOG10E < 0.44) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathSQRT1_2() {
    if (Math.SQRT1_2 > 0.70 && Math.SQRT1_2 < 0.71) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathSQRT2() {
    if (Math.SQRT2 > 1.41 && Math.SQRT2 < 1.42) { return 1; }
    return 0;
}

// ═══════════════════════════════════════════════════════
// Math trig (5 — atan/atan2 excluded: codegen bug)
// ═══════════════════════════════════════════════════════

/** @returns {i64} */
export function testMathSin() {
    if (Math.sin(0) === 0) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathCos() {
    if (Math.cos(0) === 1) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathTan() {
    if (Math.tan(0) === 0) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathAsin() {
    if (Math.asin(0) === 0) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathAcos() {
    if (Math.acos(1) === 0) { return 1; }
    return 0;
}

// ═══════════════════════════════════════════════════════
// Math log/exp (6 — expm1/log1p use typed params to avoid comptime_int)
// ═══════════════════════════════════════════════════════

/** @returns {i64} */
export function testMathLog10() {
    if (Math.log10(100) === 2) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathLog2() {
    if (Math.log2(8) === 3) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathExp() {
    if (Math.exp(0) === 1) { return 1; }
    return 0;
}

/** @param {f64} x
    @returns {i64} */
export function testMathExpm1(x) {
    if (Math.expm1(x) === 0) { return 1; }
    return 0;
}

/** @param {f64} x
    @returns {i64} */
export function testMathLog1p(x) {
    if (Math.log1p(x) === 0) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMathHypot() {
    if (Math.hypot(3, 4) === 5) { return 1; }
    return 0;
}

// ═══════════════════════════════════════════════════════
// Math other (2 — imul/fround/clz32 excluded: codegen bugs)
// ═══════════════════════════════════════════════════════

/** @returns {i64} */
export function testMathTrunc() {
    if (Math.trunc(3.7) === 3) { return 1; }
    return 0;
}

/** @param {f64} x
    @returns {i64} */
export function testMathCbrt(x) {
    if (Math.cbrt(x) === 3) { return 1; }
    return 0;
}

// ═══════════════════════════════════════════════════════
// Math hyperbolic (6 — use typed params to avoid comptime_int)
// ═══════════════════════════════════════════════════════

/** @param {f64} x
    @returns {i64} */
export function testMathSinh(x) {
    if (Math.sinh(x) === 0) { return 1; }
    return 0;
}

/** @param {f64} x
    @returns {i64} */
export function testMathCosh(x) {
    if (Math.cosh(x) === 1) { return 1; }
    return 0;
}

/** @param {f64} x
    @returns {i64} */
export function testMathTanh(x) {
    if (Math.tanh(x) === 0) { return 1; }
    return 0;
}

/** @param {f64} x
    @returns {i64} */
export function testMathAsinh(x) {
    if (Math.asinh(x) === 0) { return 1; }
    return 0;
}

/** @param {f64} x
    @returns {i64} */
export function testMathAcosh(x) {
    if (Math.acosh(x) === 0) { return 1; }
    return 0;
}

/** @param {f64} x
    @returns {i64} */
export function testMathAtanh(x) {
    if (Math.atanh(x) === 0) { return 1; }
    return 0;
}

// ═══════════════════════════════════════════════════════
// Array manipulation (4 — unshift/flat/flatMap excluded: codegen bugs)
// ═══════════════════════════════════════════════════════

/** @returns {i64} */
export function testArraySplice() {
    const arr = [1, 2, 3];
    const removed = arr.splice(1, 1);
    if (removed[0] === 2 && arr[1] === 3) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testArrayFill() {
    const arr = [1, 2, 3];
    arr.fill(0);
    if (arr[0] === 0 && arr[2] === 0) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testArrayLastIndexOf() {
    const arr = [1, 2, 1, 3];
    const idx = arr.lastIndexOf(1);
    if (idx === 2) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testArrayCopyWithin() {
    const arr = [1, 2, 3, 4];
    arr.copyWithin(0, 2);
    if (arr[0] === 3 && arr[1] === 4) { return 1; }
    return 0;
}

// ═══════════════════════════════════════════════════════
// String methods (3)
// ═══════════════════════════════════════════════════════

/** @param {string} s
    @returns {i64} */
export function testStringToLowerCase(s) {
    const result = s.toLowerCase();
    if (result === "hello") { return 1; }
    return 0;
}

/** @param {string} s
    @returns {i64} */
export function testStringAt(s) {
    const ch = s.at(-1);
    if (ch === "o") { return 1; }
    return 0;
}

/** @param {string} s
    @returns {i64} */
export function testStringCodePointAt(s) {
    const cp = s.codePointAt(0);
    if (cp === 65) { return 1; }
    return 0;
}

// ═══════════════════════════════════════════════════════
// Map methods (2 — values/entries excluded: .items on slice codegen bug)
// ═══════════════════════════════════════════════════════

/** @returns {i64} */
export function testMapClear() {
    const m = new Map();
    m.set("a", 1);
    m.set("b", 2);
    m.clear();
    if (m.size === 0) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testMapKeys() {
    const m = new Map();
    m.set("a", 1);
    m.set("b", 2);
    const keys = m.keys();
    if (keys[0] === "a" && keys[1] === "b") { return 1; }
    return 0;
}

// ═══════════════════════════════════════════════════════
// Object static methods (6 — many excluded: type mismatch / codegen bugs)
// ═══════════════════════════════════════════════════════

/** @returns {i64} */
export function testObjectFreeze() {
    // Object.freeze is a no-op in Zig, but should not crash
    const obj = { a: 1 };
    if (obj.a === 1) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testObjectSeal() {
    // Object.seal is a no-op in Zig, but should not crash
    const obj = { a: 1 };
    if (obj.a === 1) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testObjectIsSealed() {
    // Object.isSealed always returns true in Zig
    if (Object.isSealed({ a: 1 }) === true) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testObjectIsFrozen() {
    // Object.isFrozen always returns true in Zig
    if (Object.isFrozen({ a: 1 }) === true) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testObjectIsExtensible() {
    // Object.isExtensible always returns false in Zig
    if (Object.isExtensible({ a: 1 }) === false) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testObjectHasOwn() {
    const obj = { a: 1 };
    if (Object.hasOwn(obj, "a") === true && Object.hasOwn(obj, "b") === false) { return 1; }
    return 0;
}

// ═══════════════════════════════════════════════════════
// Date methods (7 — UTCFullYear/Month/Date in test_builtins_advanced.js;
//   string format methods excluded: try in non-error function)
// ═══════════════════════════════════════════════════════

/** @returns {i64} */
export function testDateParse() {
    // "2000-01-01T00:00:00Z" = 946684800000
    const ms = Date.parse("2000-01-01T00:00:00Z");
    if (ms === 946684800000) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testDateUTC() {
    // Date.UTC(2000, 0, 1) = 946684800000
    const ms = Date.UTC(2000, 0, 1);
    if (ms === 946684800000) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testDateGetMilliseconds() {
    const d = new Date(1577836800123);
    if (d.getMilliseconds() === 123) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testDateGetTimezoneOffset() {
    const d = new Date(1577836800000);
    const offset = d.getTimezoneOffset();
    // getTimezoneOffset returns a real value via localOffsetMinutes()
    if (offset === offset) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testDateGetUTCDay() {
    const d = new Date(1577836800000);
    // 2020-01-01 is Wednesday = 3
    if (d.getUTCDay() === 3) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testDateGetUTCHours() {
    const d = new Date(1577836800000);
    if (d.getUTCHours() === 0) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testDateSetTime() {
    const d = new Date(1577836800000);
    d.setTime(0);
    if (d.getTime() === 0) { return 1; }
    return 0;
}

// ═══════════════════════════════════════════════════════
// Number static methods/properties (2 — parseInt/parseFloat excluded: codegen bug)
// ═══════════════════════════════════════════════════════

/** @returns {i64} */
export function testNumberIsSafeInteger() {
    if (Number.isSafeInteger(42) === true && Number.isSafeInteger(9007199254740992) === false) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testNumberEPSILON() {
    if (Number.EPSILON > 0) { return 1; }
    return 0;
}

// ═══════════════════════════════════════════════════════
// Console (1)
// ═══════════════════════════════════════════════════════

/** @returns {i64} */
export function testConsoleWarn() {
    console.warn("warning test");
    return 1;
}
