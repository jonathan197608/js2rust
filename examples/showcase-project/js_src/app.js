// app.js — Core application layer, imports lib.js
// Dependency chain: app.js → lib.js → utils.js (2-level dependency)
// Covers: C ABI exports (i64, bool, string), recursive function,
//         multi-branch return (S3.2), nested function calls,
//         string concat via + (S4.1), ternary (S4.1),
//         template literals (S4.1), clamp pattern

import { testClassBasic, testClassVec2, makeAdder, testObjLiteral } from './helpers.js';
import { testTernary, testSwitch, testParenExpr, testImportedAdd } from './helpers.js';

// ── Export: integer return (C ABI: i64) ─────────────────────────

export function showcaseSum(a, b) {
    return a + b;
}

export function showcaseFactorial(n) {
    if (n <= 1) { return 1; }
    return n * showcaseFactorial(n - 1);
}

export function showcaseMul(a, b) {
    return a * b;
}

// ── Export: string return (C ABI: [*:0]const u8) ────────────────

export function showcaseGreet(name) {
    return "Hello, " + name + "!";
}

// ── Export: bool return (C ABI: bool) ───────────────────────────

export function showcaseIsPositive(x) {
    return x > 0;
}

// ── Multiple return branches (S3.2) — proven ────────────────────

export function testMultiBranch(x) {
    if (x > 100) { return 3; }
    if (x > 50) { return 2; }
    if (x > 0) { return 1; }
    return 0;
}

// ── Nested function calls ───────────────────────────────────────

function helper(x) { return x * 2; }
function doubleHelper(x) { return helper(helper(x)); }

export function testNestedCalls() {
    return doubleHelper(5);
}

// ── Clamp function (multi-branch + comparison) ──────────────────

export function testClamp(x, lo, hi) {
    if (x < lo) { return lo; }
    if (x > hi) { return hi; }
    return x;
}

// ── Absolute value via ternary ──────────────────────────────────

export function testAbsTernary(x) {
    return x >= 0 ? x : -x;
}

// ── Min/max via multi-branch return (avoid const→JsAny issue) ───

export function testMin(a, b) {
    if (a < b) { return a; }
    return b;
}

export function testMax(a, b) {
    if (a > b) { return a; }
    return b;
}

// ── Template expression with numeric calc ───────────────────────

export function testTemplate(x, y) {
    return `result=${x + y}`;
}

// ── Sign function — multi-branch integer ────────────────────────

export function testSign(x) {
    if (x > 0) { return 1; }
    if (x < 0) { return -1; }
    return 0;
}

// ── Integration: nested call expression (avoids const→JsAny) ────
// The 2-level dependency chain (app → lib → utils) is validated by
// the Zig compilation itself — all imports resolve correctly.
// Here we verify that exported functions compute correctly.

export function runAllTests() {
    // showcaseSum(3, 7) = 10, testMultiBranch(75) = 2 → 10 + 2 = 12
    return showcaseSum(showcaseSum(3, 7), testMultiBranch(75));
}

// ══════════════════════════════════════════════════════════════
// Phase 1: Loops — for / while / do-while / for-of / break / continue
// [LOCKED] Do not modify once tests pass.
// ══════════════════════════════════════════════════════════════

// -- C-style for loop: sum 1..n --
export function forSum(n) {
    let sum = 0;
    for (let i = 1; i <= n; i++) {
        sum = sum + i;
    }
    return sum;
}

// -- while loop: count iterations halving n --
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
export function doWhileOnce() {
    let count = 0;
    do {
        count = count + 1;
    } while (false);
    return count;
}

// -- for-of: sum static array elements --
export function forOfSum() {
    const arr = [10, 20, 30, 40];
    let sum = 0;
    for (const item of arr) {
        sum = sum + item;
    }
    return sum;
}

// -- break: exit loop early when threshold reached --
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

// ══════════════════════════════════════════════════════════════
// Phase 2: Error Handling — try-catch / throw
// [LOCKED] Do not modify once tests pass.
// ══════════════════════════════════════════════════════════════

// -- Basic throw + catch: catch path taken --
export function tryCatchBasic() {
    try {
        throw "error";
    } catch (e) {
        return 42;
    }
}

// -- Side effect before throw preserved in catch --
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
