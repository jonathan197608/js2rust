// helpers.js — Mid-level layer, imports utils.js
// Covers: classes (S6.4), closures (S8), object literals (S4.1),
//         template literals (S4.1),
//         control flow (S5.1): if/else, switch,
//         ternary (S4.1), parenthesized expr (S4.1),
//         multi-variable decl (S5.3), boolean literals (S4.1),
//         block statement (S5.1),
//         dynamic arrays — push/pop (S7.4), imported function usage

import { add, mathSqrt, compareLt, withDefault, hexNum } from './utils.js';

// ── Classes (S6.4) ──────────────────────────────────────────────

// Basic class with constructor + instance method
class Rectangle {
    constructor(w, h) {
        this.width = w;
        this.height = h;
    }
    area() {
        return this.width * this.height;
    }
}

// Class with field defaults (no mutation after construction)
class Settings {
    constructor() {
        this.volume = 50;
        this.brightness = 75;
    }
    getVolume() { return this.volume; }
    getBrightness() { return this.brightness; }
}

// Multiple instance methods
class Vec2 {
    constructor(x, y) {
        this.x = x;
        this.y = y;
    }
    sum() { return this.x + this.y; }
    product() { return this.x * this.y; }
}

// Static method
class PointFactory {
    constructor(x, y) {
        this.x = x;
        this.y = y;
    }
    static create(x, y) {
        return new PointFactory(x, y);
    }
    getX() { return this.x; }
    getY() { return this.y; }
}

export function testClassBasic() {
    const r = new Rectangle(3, 4);
    return r.area();
}

export function testClassSettings() {
    const s = new Settings();
    return s.getVolume();
}

export function testClassVec2() {
    const v = new Vec2(3, 7);
    return v.sum();
}

export function testClassVec2Product() {
    const v = new Vec2(5, 6);
    return v.product();
}

export function testClassStatic() {
    const p = PointFactory.create(10, 20);
    return p.getX() + p.getY();
}

// ── Closures (S8) ───────────────────────────────────────────────

// Single capture
export function makeAdder(x) {
    const adder = (y) => x + y;
    return adder(10);
}

// Multiple captures
export function makeScaler(factor, offset) {
    const scale = (x) => x * factor + offset;
    return scale(5);
}

// Nested closure
export function nestedClosure(a) {
    const inner = (b) => a * b;
    return inner(4);
}

// ── Object literal (S4.1) ───────────────────────────────────────

export function testObjLiteral() {
    const obj = { x: 10, y: 20, z: 30 };
    return obj.x + obj.y + obj.z;
}

export function testObjAccess() {
    const point = { x: 5, y: 8 };
    return point.x * point.y;
}

// ── Template literals (S4.1) — proven via test_template.js ──────

// Dynamic array element interpolation
export function tplDynArr() {
    const arr = [];
    arr.push(10);
    arr.push(20);
    arr.push(30);
    return `first=${arr[0]},last=${arr[2]}`;
}

// Object property interpolation
export function tplObjProp() {
    const point = { x: 10, y: 20 };
    return `(${point.x},${point.y})`;
}

// Multi-line with interpolation
export function tplMultiLine() {
    const a = 100;
    const b = 200;
    return `a=${a}
b=${b}
sum=${a + b}`;
}

// Plain template (no interpolation)
export function tplPlain() {
    return `hello world`;
}

// Array math inside template
export function tplArrMath() {
    const arr = [];
    arr.push(3);
    arr.push(7);
    return `sum=${arr[0] + arr[1]}`;
}

// ── Control flow (S5.1) ─────────────────────────────────────────

// if-else chain (proven)
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
export function testSwitch(x) {
    switch (x) {
        case 1: return 10;
        case 2: return 20;
        case 3: return 30;
        default: return 0;
    }
}

// ── Ternary operator (S4.1) — proven ────────────────────────────

export function testTernary(x) {
    return x > 0 ? x : -x;
}

// Nested ternary
export function testNestedTernary(x) {
    return x > 0 ? 1 : x < 0 ? -1 : 0;
}

// ── Parenthesized expression (S4.1) — proven ────────────────────

export function testParenExpr(a, b) {
    return (a + b) * 2;
}

// ── Multi-variable declaration (S5.3) — proven ──────────────────

export function testMultiDecl() {
    const a = 1, b = 2, c = 3;
    return a + b + c;
}

// ── Boolean/null literals (S4.1) ────────────────────────────────

export function testBoolLiteral() {
    const t = true;
    if (t) { return 1; }
    return 0;
}

// ── Block statement (S5.1) ──────────────────────────────────────

export function testBlock() {
    {
        const x = 42;
        return x;
    }
}

// ── Dynamic arrays — push/pop (S7.4) ────────────────────────────
// Proven pattern: push + index access (from test_template.js)

export function testDynArrayPush() {
    const arr = [];
    arr.push(10);
    arr.push(20);
    arr.push(30);
    return arr[0] + arr[1] + arr[2];
}

// ── Using imported utils functions ──────────────────────────────

export function testImportedAdd() {
    return add(10, 20);
}

export function testImportedSqrt() {
    return mathSqrt(16.0);
}

export function testImportedDefault() {
    return withDefault(5, 15);
}

export function testImportedHex() {
    return hexNum();
}

export function testImportedCompare() {
    return compareLt(3, 5);
}
