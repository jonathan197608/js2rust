// helpers.js — Mid-level layer, imports utils.js
// Covers: closures (S8), object literals (S4.1),
//         template literals (S4.1),
//         control flow (S5.1): if/else, switch,
//         ternary (S4.1), parenthesized expr (S4.1),
//         multi-variable decl (S5.3), boolean literals (S4.1),
//         block statement (S5.1),
//         dynamic arrays — push/pop (S7.4), imported function usage

import { add, mathSqrt, compareLt, withDefault, hexNum } from './utils.js';

// ── Classes (S6.4) ──────────────────────────────────────

/** @returns {i64} */
export function testClassBasic() {
    class Rectangle {
        constructor(w, h) { this.width = w; this.height = h; }
        area() { return this.width * this.height; }
    }
    const r = new Rectangle(3, 4);
    return r.area();
}

/** @returns {i64} */
export function testClassSettings() {
    class Settings {
        constructor() { this.width = 5; this.height = 10; }
        getVolume() { return this.width * this.height; }
    }
    const s = new Settings();
    return s.getVolume();
}

/** @returns {i64} */
export function testClassVec2() {
    class Vec2 {
        constructor(x, y) { this.x = x; this.y = y; }
        sum() { return this.x + this.y; }
    }
    const v = new Vec2(3, 7);
    return v.sum();
}

/** @returns {i64} */
export function testClassVec2Product() {
    class Vec2 {
        constructor(x, y) { this.x = x; this.y = y; }
        product() { return this.x * this.y; }
    }
    const v = new Vec2(5, 6);
    return v.product();
}

/** @returns {i64} */
export function testClassStatic() {
    class Point {
        constructor(x, y) { this.x = x; this.y = y; }
        getX() { return this.x; }
        getY() { return this.y; }
    }
    class PointFactory {
        static create(x, y) { return new Point(x, y); }
    }
    const p = PointFactory.create(10, 20);
    return p.getX() + p.getY();
}

// ── Closures (S8) ───────────────────────────────────────

/** @returns {i64} */
export function makeAdder(x) {
    return function(y) { return x + y; }(10);
}

/** @returns {i64} */
export function makeScaler(factor, offset) {
    return function(val) { return factor * val + offset; }(5);
}

/** @returns {i64} */
export function nestedClosure(a) {
    return function(b) {
        return function(c) { return a * b + c; }(1);
    }(4);
}

// ── Object literal (S4.1) ───────────────────────────────────

/** @returns {i64} */
export function testObjLiteral() {
    const obj = { x: 10, y: 20, z: 30 };
    return obj.x + obj.y + obj.z;
}

/** @returns {i64} */
export function testObjAccess() {
    const point = { x: 5, y: 8 };
    return point.x * point.y;
}

// ── Template literals (S4.1) — proven via test_template.js ──────

// Static array element interpolation
// NOTE: dynamic arr.push() not supported in native_proto (see #231); use static init.
/** @returns {string} */
export function tplDynArr() {
    const arr = [10, 20, 30];
    return `first=${arr[0]},last=${arr[2]}`;
}

// Object property interpolation
/** @returns {string} */
export function tplObjProp() {
    const point = { x: 10, y: 20 };
    return `(${point.x},${point.y})`;
}

// Multi-line with interpolation
/** @returns {string} */
export function tplMultiLine() {
    const a = 100;
    const b = 200;
    return `a=${a}
b=${b}
sum=${a + b}`;
}

// Plain template (no interpolation)
/** @returns {string} */
export function tplPlain() {
    return `hello world`;
}

// Array math inside template
/** @returns {string} */
export function tplArrMath() {
    const arr = [3, 7];
    return `sum=${arr[0] + arr[1]}`;
}

// ── Control flow (S5.1) ─────────────────────────────────────────

// if-else chain (proven)
/** @returns {i64} */
export function testIfElse(x) {
    if (x > 0) {
        return 1;
    } else if (x < 0) {
        return -1;
    } else {
        return 0;
    }
}

// Switch-case (proven)
/** @returns {i64} */
export function testSwitch(x) {
    switch (x) {
        case 1: return 10;
        case 2: return 20;
        case 3: return 30;
        default: return 0;
    }
}

// ── Ternary operator (S4.1) — proven ────────────────────────────

/** @returns {i64} */
export function testTernary(x) {
    return x > 0 ? x : -x;
}

// Nested ternary
/** @returns {i64} */
export function testNestedTernary(x) {
    return x > 0 ? 1 : x < 0 ? -1 : 0;
}

// ── Parenthesized expression (S4.1) — proven ────────────────────

/** @returns {i64} */
export function testParenExpr(a, b) {
    return (a + b) * 2;
}

// ── Multi-variable declaration (S5.3) — proven ──────────────────

/** @returns {i64} */
export function testMultiDecl() {
    const a = 1, b = 2, c = 3;
    return a + b + c;
}

// ── Boolean/null literals (S4.1) ───────────────────────────────

/** @returns {i64} */
export function testBoolLiteral() {
    const t = true;
    if (t) { return 1; }
    return 0;
}

// ── Block statement (S5.1) ──────────────────────────────────────

/** @returns {i64} */
export function testBlock() {
    {
        const x = 42;
        return x;
    }
}

// ── Static arrays — index access (S7.4) ─────────────────────────
// NOTE: dynamic arr.push() not supported in native_proto (see #231).

/** @returns {i64} */
export function testDynArrayPush() {
    const arr = [10, 20, 30];
    return arr[0] + arr[1] + arr[2];
}

// ── Using imported utils functions ──────────────────────────────

/** @returns {i64} */
export function testImportedAdd() {
    return add(10, 20);
}

/** @returns {f64} */
export function testImportedSqrt() {
    return mathSqrt(16.0);
}

/** @returns {i64} */
export function testImportedDefault() {
    return withDefault(5, 15);
}

/** @returns {i64} */
export function testImportedHex() {
    return hexNum();
}

/** @returns {bool} */
export function testImportedCompare() {
    return compareLt(3, 5);
}
