// utils.js — Leaf dependency: foundational utilities
// Covers: Math builtins (S7.1), global functions (S7.2), NaN/Infinity (S4.1),
//         arithmetic/comparison/unary/bitwise operators (S4.1),
//         string methods (S7.3), static array methods (S7.4),
//         default parameters (S6.3), type inference basics (S3)

// ── Math builtins (S7.1) ────────────────────────────────────────
// All use float params to avoid integer/float codegen issues

export function mathFloor(x) { return Math.floor(x); }
export function mathCeil(x) { return Math.ceil(x); }
export function mathRound(x) { return Math.round(x); }
export function mathTrunc(x) { return Math.trunc(x); }
export function mathSqrt(x) { return Math.sqrt(x); }
export function mathSin(x) { return Math.sin(x); }
export function mathCos(x) { return Math.cos(x); }
export function mathTan(x) { return Math.tan(x); }
export function mathExp(x) { return Math.exp(x); }
export function mathLog(x) { return Math.log(x); }
export function mathLog2(x) { return Math.log2(x); }
export function mathLog10(x) { return Math.log10(x); }
export function mathPow(b, e) { return Math.pow(b, e); }
export function mathAbs(x) { return Math.abs(x); }

export function mathMinMax(a, b) {
    return Math.min(a, b) + Math.max(a, b);
}

export function mathHypot(a, b) {
    return Math.hypot(a, b);
}

export function mathSign(x) {
    if (x > 0) return 1;
    if (x < 0) return -1;
    return 0;
}

export function mathConstants() {
    const pi = Math.PI;
    const e = Math.E;
    return pi + e;
}

// ── Global functions (S7.2) ─────────────────────────────────────

export function globalParseInt(s) {
    return parseInt(s);
}

// ── Arithmetic operators (S4.1) — add, sub, mul only ────────────

export function add(a, b) { return a + b; }
export function sub(a, b) { return a - b; }
export function mul(a, b) { return a * b; }

// ── Bitwise operators (S4.1) — single-op functions ──────────────

export function bitwiseAnd(a, b) { return a & b; }
export function bitwiseOr(a, b) { return a | b; }
export function bitwiseXor(a, b) { return a ^ b; }
export function bitwiseNot(a) { return ~a; }
export function bitwiseLshift(a, b) { return a << b; }
export function bitwiseRshift(a, b) { return a >> b; }

// ── Comparison operators (S4.1) ─────────────────────────────────

export function compareLt(a, b) { return a < b; }
export function compareGt(a, b) { return a > b; }
export function compareLe(a, b) { return a <= b; }
export function compareGe(a, b) { return a >= b; }
export function compareEq(a, b) { return a === b; }
export function compareNeq(a, b) { return a !== b; }

// ── Unary operators (S4.1) ──────────────────────────────────────

export function unaryNeg(x) { return -x; }

// ── String methods (S7.3) — inline constants (proven pattern) ───

export function strLength() {
    const s = "hello world";
    return s.length;
}

export function strToUpper() {
    const s = "hello";
    return s.toUpperCase();
}

export function strToLower() {
    const s = "HELLO";
    return s.toLowerCase();
}

export function strIncludes() {
    const s = "hello world";
    return s.includes("world");
}

export function strIndexOf() {
    const s = "hello world";
    return s.indexOf("world");
}

export function strStartsWith() {
    const s = "hello world";
    return s.startsWith("hello");
}

export function strEndsWith() {
    const s = "hello world";
    return s.endsWith("world");
}

export function strTrim() {
    const s = "  hello  ";
    return s.trim();
}

export function strSlice() {
    const s = "hello world";
    return s.slice(0, 5);
}

// ── Static array methods (S7.4) ─────────────────────────────────

export function arrLength() {
    const arr = [10, 20, 30, 40, 50];
    return arr.length;
}

export function arrIncludes() {
    const arr = [10, 20, 30];
    return arr.includes(20);
}

export function arrIndexOf() {
    const arr = [10, 20, 30];
    return arr.indexOf(20);
}

export function arrAccess() {
    const arr = [10, 20, 30];
    return arr[1];
}

// ── Default parameters (S6.3) ───────────────────────────────────

export function withDefault(x, y = 10) {
    return x + y;
}

// ── Type inference basics (S3) ──────────────────────────────────

export function intLiteral() { return 42; }
export function floatLiteral() { return 3.14; }
export function boolLiteral() { return true; }
export function strLiteral() { return "hello"; }

// ── Hex literal (S4.1) ──────────────────────────────────────────

export function hexNum() { return 0xFF; }

// ── Function return type tracking (S3.2) ────────────────────────

export function getNumber() { return 42; }
export function wrapper() {
    const result = getNumber();
    return result + 1;
}
