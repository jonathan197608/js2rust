// utils.js — Leaf dependency: foundational utilities
// Covers: Math builtins (S7.1), global functions (S7.2), NaN/Infinity (S4.1),
//         arithmetic/comparison/unary/bitwise operators (S4.1),
//         string methods (S7.3), static array methods (S7.4),
//         default parameters (S6.3), type inference basics (S3)

// ── Math builtins (S7.1) ────────────────────────────────────────
// All use float params to avoid integer/float codegen issues

/**
 * @returns {f64}
 */
export function mathFloor(x) { return Math.floor(x); }
/**
 * @returns {f64}
 */
export function mathCeil(x) { return Math.ceil(x); }
/**
 * @returns {f64}
 */
export function mathRound(x) { return Math.round(x); }
// Math.trunc not supported in native_proto
// export function mathTrunc(x) { return Math.trunc(x); }
/**
 * @returns {f64}
 */
export function mathSqrt(x) { return Math.sqrt(x); }
// Math.sin not supported in native_proto
// export function mathSin(x) { return Math.sin(x); }
// Math.cos not supported in native_proto
// export function mathCos(x) { return Math.cos(x); }
// Math.tan not supported in native_proto
// export function mathTan(x) { return Math.tan(x); }
// Math.exp not supported in native_proto
// export function mathExp(x) { return Math.exp(x); }
// Math.log not supported in native_proto
// export function mathLog(x) { return Math.log(x); }
// Math.log2 not supported in native_proto
// export function mathLog2(x) { return Math.log2(x); }
// Math.log10 not supported in native_proto
// export function mathLog10(x) { return Math.log10(x); }
/** @returns {f64} */
export function mathPow(b, e) { return Math.pow(b, e); }
/** @returns {f64} */
export function mathAbs(x) { return Math.abs(x); }

/** @returns {f64} */
export function mathMinMax(a, b) {
    return Math.min(a, b) + Math.max(a, b);
}

// Math.hypot not supported in native_proto
// export function mathHypot(a, b) {
//     return Math.hypot(a, b);
// }

/** @returns {i64} */
export function mathSign(x) {
    if (x > 0) return 1;
    if (x < 0) return -1;
    return 0;
}

/** @returns {f64} */
export function mathConstants() {
    const pi = Math.PI;
    const e = Math.E;
    return pi + e;
}

// ── Global functions (S7.2) ─────────────────────────────────────

/** @returns {i64} */
export function globalParseInt(s) {
    return parseInt(s);
}

// ── Arithmetic operators (S4.1) — add, sub, mul only ────────────

/** @returns {i64} */
export function add(a, b) { return a + b; }
/** @returns {i64} */
export function sub(a, b) { return a - b; }
/** @returns {i64} */
export function mul(a, b) { return a * b; }

// ── Bitwise operators (S4.1) — single-op functions ──────────────

/** @returns {i64} */
export function bitwiseAnd(a, b) { return a & b; }
/** @returns {i64} */
export function bitwiseOr(a, b) { return a | b; }
/** @returns {i64} */
export function bitwiseXor(a, b) { return a ^ b; }
// bitwise NOT (~) not supported in native_proto
// export function bitwiseNot(a) { return ~a; }
/** @returns {i64} */
export function bitwiseLshift(a, b) { return a << b; }
/** @returns {i64} */
export function bitwiseRshift(a, b) { return a >> b; }

// ── Comparison operators (S4.1) ─────────────────────────────────

/** @returns {bool} */
export function compareLt(a, b) { return a < b; }
/** @returns {bool} */
export function compareGt(a, b) { return a > b; }
/** @returns {bool} */
export function compareLe(a, b) { return a <= b; }
/** @returns {bool} */
export function compareGe(a, b) { return a >= b; }
/** @returns {bool} */
export function compareEq(a, b) { return a === b; }
/** @returns {bool} */
export function compareNeq(a, b) { return a !== b; }

// ── Unary operators (S4.1) ──────────────────────────────────────

/** @returns {i64} */
export function unaryNeg(x) { return -x; }

// ── String methods (S7.3) — inline constants (proven pattern) ───

/** @returns {i64} */
export function strLength() {
    const s = "hello world";
    return s.length;
}

// String.toUpperCase() not supported in native_proto
// export function strToUpper() {
//     const s = "hello";
//     return s.toUpperCase();
// }

// String.toLowerCase() not supported in native_proto
// export function strToLower() {
//     const s = "HELLO";
//     return s.toLowerCase();
// }

/** @returns {bool} */
export function strIncludes() {
    const s = "hello world";
    return s.includes("world");
}

/** @returns {i64} */
export function strIndexOf() {
    const s = "hello world";
    return s.indexOf("world");
}

/** @returns {bool} */
export function strStartsWith() {
    const s = "hello world";
    return s.startsWith("hello");
}

/** @returns {bool} */
export function strEndsWith() {
    const s = "hello world";
    return s.endsWith("world");
}

// String.trim() not supported in native_proto
// export function strTrim() {
//     const s = "  hello  ";
//     return s.trim();
// }

// String.slice() not supported in native_proto
// export function strSlice() {
//     const s = "hello world";
//     return s.slice(0, 5);
// }

// ── Static array methods (S7.4) ─────────────────────────────────

/** @returns {i64} */
export function arrLength() {
    const arr = [10, 20, 30, 40, 50];
    return arr.length;
}

/** @returns {bool} */
export function arrIncludes() {
    const arr = [10, 20, 30];
    return arr.includes(20);
}

/** @returns {i64} */
export function arrIndexOf() {
    const arr = [10, 20, 30];
    return arr.indexOf(20);
}

/** @returns {i64} */
export function arrAccess() {
    const arr = [10, 20, 30];
    return arr[1];
}

// ── Default parameters (S6.3) ───────────────────────────────────

/** @returns {i64} */
export function withDefault(x, y = 10) {
    return x + y;
}

// ── Type inference basics (S3) ──────────────────────────────────

/** @returns {i64} */
export function intLiteral() { return 42; }
/** @returns {f64} */
export function floatLiteral() { return 3.14; }
/** @returns {bool} */
export function boolLiteral() { return true; }
/** @returns {str} */
export function strLiteral() { return "hello"; }

// ── Hex literal (S4.1) ──────────────────────────────────────────

/** @returns {i64} */
export function hexNum() { return 0xFF; }

// ── Function return type tracking (S3.2) ────────────────────────

/** @returns {i64} */
export function getNumber() { return 42; }
/** @returns {i64} */
export function wrapper() {
    const result = getNumber();
    return result + 1;
}
