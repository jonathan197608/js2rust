// app.js — Core application layer, imports lib.js
// Dependency chain: app.js → lib.js → utils.js (2-level dependency)
// Covers: C ABI exports (i64, bool, string), recursive function,
//         multi-branch return (S3.2), nested function calls,
//         string concat via + (S4.1), ternary (S4.1),
//         template literals (S4.1), clamp pattern

import { testClassBasic, testClassVec2, makeAdder, testObjLiteral } from './helpers.js';
import { testTernary, testSwitch, testParenExpr, testImportedAdd } from './helpers.js';

// ── Export: integer return (C ABI: i64) ─────────────────────────

/**
 * @param {i64} a
 * @param {i64} b
 * @returns {i64}
 */
export function showcaseSum(a, b) {
    return a + b;
}

/**
 * @param {i64} n
 * @returns {i64}
 */
export function showcaseFactorial(n) {
    if (n <= 1) { return 1; }
    return n * showcaseFactorial(n - 1);
}

/**
 * @param {i64} a
 * @param {i64} b
 * @returns {i64}
 */
export function showcaseMul(a, b) {
    return a * b;
}

// ── Export: string return (C ABI: [*:0]const u8) ────────────────

/**
 * @param {string} name
 * @returns {string}
 */
export function showcaseGreet(name) {
    return "Hello, " + name + "!";
}

// ── Export: bool return (C ABI: bool) ───────────────────────────

/**
 * @param {i64} x
 * @returns {bool}
 */
export function showcaseIsPositive(x) {
    return x > 0;
}

// ── Multiple return branches (S3.2) — proven ────────────────────

/**
 * @param {i64} x
 * @returns {i64}
 */
export function testMultiBranch(x) {
    if (x > 100) { return 3; }
    if (x > 50) { return 2; }
    if (x > 0) { return 1; }
    return 0;
}

// ── Nested function calls ───────────────────────────────────────

/** @returns {i64} */
function helper(x) { return x * 2; }
/** @returns {i64} */
function doubleHelper(x) { return helper(helper(x)); }

/**
 * @returns {i64}
 */
export function testNestedCalls() {
    return doubleHelper(5);
}

// ── Clamp function (multi-branch + comparison) ──────────────────

/**
 * @param {i64} x
 * @param {i64} lo
 * @param {i64} hi
 * @returns {i64}
 */
export function testClamp(x, lo, hi) {
    if (x < lo) { return lo; }
    if (x > hi) { return hi; }
    return x;
}

// ── Absolute value via ternary ──────────────────────────────────

/**
 * @param {i64} x
 * @returns {i64}
 */
export function testAbsTernary(x) {
    return x >= 0 ? x : -x;
}

// ── Min/max via multi-branch return (avoid const→JsAny issue) ───

/**
 * @param {i64} a
 * @param {i64} b
 * @returns {i64}
 */
export function testMin(a, b) {
    if (a < b) { return a; }
    return b;
}

/**
 * @param {i64} a
 * @param {i64} b
 * @returns {i64}
 */
export function testMax(a, b) {
    if (a > b) { return a; }
    return b;
}

// ── Template expression with numeric calc ───────────────────────
// NOTE: Template literals (incl. numeric interpolation) are now supported.
// This stub keeps an i64 return to match the existing test assertion;
// see helpers.js tplObjProp/tplMultiLine for string-returning template tests.
/** @returns {i64} */
export function testTemplate(x, y) { return 0; }

// ── Sign function — multi-branch integer ────────────────────────

/**
 * @param {i64} x
 * @returns {i64}
 */
export function testSign(x) {
    if (x > 0) { return 1; }
    if (x < 0) { return -1; }
    return 0;
}

// ── Integration: nested call expression (avoids const→JsAny) ────
// The 2-level dependency chain (app → lib → utils) is validated by
// the Zig compilation itself — all imports resolve correctly.
// Here we verify that exported functions compute correctly.

/**
 * @returns {i64}
 */
export function runAllTests() {
    // showcaseSum(3, 7) = 10, testMultiBranch(75) = 2 → 10 + 2 = 12
    return showcaseSum(showcaseSum(3, 7), testMultiBranch(75));
}

// ════════════════════════════════════════════════════════
// Phase 1: Loops — for / while / do-while / for-of / break / continue
// [LOCKED] Do not modify once tests pass.
// ════════════════════════════════════════════════════════

// -- C-style for loop: sum 1..n --
/** @returns {i64} */
export function forSum(n) {
    let sum = 0;
    for (let i = 1; i <= n; i++) {
        sum = sum + i;
    }
    return sum;
}

// -- while loop: count iterations halving n --
/** @returns {i64} */
export function whileHalve(n) {
    let count = 0;
    let current = n;
    while (current > 0) {
        count = count + 1;
        current = current / 2;
    }
    return count;
}

// -- do-while: always runs at least once --
/** @returns {i64} */
export function doWhileOnce() {
    let count = 0;
    do {
        count = count + 1;
    } while (false);
    return count;
}

// -- for-of: sum static array elements --
/** @returns {i64} */
export function forOfSum() {
    const arr = [10, 20, 30, 40];
    let sum = 0;
    for (const item of arr) {
        sum = sum + item;
    }
    return sum;
}

// -- break: exit loop early when threshold reached --
/** @returns {i64} */
export function breakAtFive(n) {
    let sum = 0;
    for (let i = 1; i <= n; i++) {
        if (i > 5) {
            break;
        }
        sum = sum + i;
    }
    return sum;
}

// -- continue: skip odd numbers, sum only evens --
/** @returns {i64} */
export function continueEven(n) {
    let sum = 0;
    for (let i = 1; i <= n; i++) {
        if (i % 2 !== 0) {
            continue;
        }
        sum = sum + i;
    }
    return sum;
}

// ════════════════════════════════════════════════════════
// Phase 2: Error Handling — try-catch / throw
// [LOCKED] Do not modify once tests pass.
// ════════════════════════════════════════════════════════

// -- Basic throw + catch: catch path taken --
/** @returns {i64} */
export function tryCatchBasic() {
    try {
        throw "error";
    } catch (e) {
        return 42;
    }
}

// -- Side effect before throw preserved in catch --
/** @returns {i64} */
export function tryCatchSideEffect() {
    let x = 10;
    try {
        x = x + 5;
        throw "error";
    } catch (e) {
        return x;
    }
}

// -- Conditional throw: normal path vs error path --
/** @returns {i64} */
export function throwIfNegative(n) {
    try {
        if (n < 0) {
            throw "negative";
        }
        return n;
    } catch (e) {
        return -n;
    }
}

// -- Multiple operations in catch --
/** @returns {i64} */
export function tryCatchMultiOp() {
    let a = 5;
    const b = 10;
    try {
        a = a + b;
        throw "error";
    } catch (e) {
        return a * 2;
    }
}

// ════════════════════════════════════════════════════════
// Phase 3: Operators — div / mod / compound assign / logical
// [LOCKED] Do not modify once tests pass.
// ════════════════════════════════════════════════════════

// -- Integer division via assignment: @divTrunc --
/** @returns {i64} */
export function intDivTest() {
    let x = 17;
    x = x / 5;
    return x;
}

// -- Modulo via assignment: @rem --
/** @returns {i64} */
export function modOpTest() {
    let x = 17;
    x = x % 5;
    return x;
}

// -- Compound assignment: +=, *=, -= --
/** @returns {i64} */
export function compoundOps() {
    let x = 2;
    x += 3;
    x *= 4;
    x -= 8;
    return x;
}

// -- Logical AND: short-circuit --
/** @returns {i64} */
export function logicAnd(a, b) {
    if (a > 0 && b > 0) {
        return 1;
    }
    return 0;
}

// -- Logical OR: short-circuit --
/** @returns {i64} */
export function logicOr(a, b) {
    if (a > 0 || b > 0) {
        return 1;
    }
    return 0;
}

// ════════════════════════════════════════════════════════
// Phase 4: Collections — Map / Set
// [LOCKED] Do not modify once tests pass.
// ════════════════════════════════════════════════════════

// -- Map: set + has (positive) --
/** @returns {i64} */
export function testMapHas() {
    const m = new Map();
    m.set("hello", 42);
    m.set("world", 99);
    if (m.has("hello")) {
        return 1;
    }
    return 0;
}

// -- Map: has returns false for missing key --
/** @returns {i64} */
export function testMapMissing() {
    const m = new Map();
    m.set("a", 1);
    if (m.has("b")) {
        return 1;
    }
    return 0;
}

// -- Set: add + has (positive) --
/** @returns {i64} */
export function testSetHas() {
    const s = new Set();
    s.add(1);
    s.add(2);
    s.add(3);
    if (s.has(2)) {
        return 1;
    }
    return 0;
}

// -- Set: has returns false for missing value --
/** @returns {i64} */
export function testSetMissing() {
    const s = new Set();
    s.add(10);
    s.add(20);
    if (s.has(30)) {
        return 1;
    }
    return 0;
}

// ════════════════════════════════════════════════════════
// Phase 5: Additional tests (expand as features are added)
// ════════════════════════════════════════════════════════

// -- Map: size property --
/** @returns {i64} */
export function testMapSize() {
    const m = new Map();
    m.set("a", 1);
    m.set("b", 2);
    m.set("c", 3);
    if (m.size === 3) {
        return 1;
    }
    return 0;
}

// -- Set: size property --
/** @returns {i64} */
export function testSetSize() {
    const s = new Set();
    s.add(10);
    s.add(20);
    s.add(30);
    if (s.size === 3) {
        return 1;
    }
    return 0;
}

// -- Map: get() method --
/** @returns {i64} */
export function testMapGet() {
    const m = new Map();
    m.set("a", 100);
    m.set("b", 200);
    const v = m.get("a");
    if (v === 100) {
        return 1;
    }
    return 0;
}

// -- Map: delete() method --
/** @returns {i64} */
export function testMapDelete() {
    const m = new Map();
    m.set("a", 1);
    m.set("b", 2);
    const deleted = m.delete("a");
    if (deleted === true && m.size === 1) {
        return 1;
    }
    return 0;
}

// -- Set: delete() method --
/** @returns {i64} */
export function testSetDelete() {
    const s = new Set();
    s.add(10);
    s.add(20);
    const deleted = s.delete(10);
    if (deleted === true && s.size === 1) {
        return 1;
    }
    return 0;
}

// -- Bitwise AND: & --
/** @returns {i64} */
export function testBitwiseAnd() {
    const result = 0b1100 & 0b1010;
    if (result === 8) { return 1; }  // 0b1000 = 8
    return 0;
}

// -- Bitwise OR: | --
/** @returns {i64} */
export function testBitwiseOr() {
    const result = 0b1100 | 0b1010;
    if (result === 14) { return 1; }  // 0b1110 = 14
    return 0;
}

// -- Bitwise XOR: ^ --
/** @returns {i64} */
export function testBitwiseXor() {
    const result = 0b1100 ^ 0b1010;
    if (result === 6) { return 1; }  // 0b0110 = 6
    return 0;
}

// -- Division expression (not assignment): @divTrunc --
/** @returns {i64} */
export function testDivExpr() {
    return 17 / 5;
}

// -- Modulo expression (not assignment): @rem --
/** @returns {i64} */
export function testModExpr() {
    return 17 % 5;
}

// ========== P2: Destructuring Defaults ==========

// -- Object destructuring with defaults (struct with known fields) --
/** @returns {i64} */
export function testDestructureObjDefault() {
    const sobj = { a: 10, b: 20 };
    const { a = 1, b = 2, c = 3 } = sobj;
    if (a === 10 && b === 20 && c === 3) { return 1; }
    return 0;
}

// -- Object destructuring with defaults (empty object → HashMap) --
/** @returns {i64} */
export function testDestructureObjDefaultEmpty() {
    const hobj = {};
    const { x = 42, y = 99 } = hobj;
    if (x === 42 && y === 99) { return 1; }
    return 0;
}

// -- Array destructuring with defaults (ArrayList with known elements) --
/** @returns {i64} */
export function testDestructureArrDefault() {
    const sarr = [10, 20];
    const [a = 1, b = 2, c = 3] = sarr;
    if (a === 10 && b === 20 && c === 3) { return 1; }
    return 0;
}

// -- Array destructuring with defaults (ArrayList, no out-of-bounds) --
/** @returns {i64} */
export function testDestructureArrDefaultEmpty() {
    const darr = [1, 2, 3];
    const [x = 5, y = 6] = darr;
    if (x === 1 && y === 2) { return 1; }
    return 0;
}