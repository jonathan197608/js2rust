// test_expr_advanced.js
// End-to-end tests for expression features: unary plus, bitwise compound assignment.
// NOTE: ?? / ??= / &&= / ||= are in test_nullish_ops.js (separated to avoid
// JsAny type pollution — null/undefined in this file causes all vars to become
// JsAny, breaking i64 bitwise operations).

// ══════════════════════════════════════════════════════════════
// Section 2.6: Unary plus +x
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testUnaryPlus() {
    const x = 5;
    const y = +x;
    if (y === 5) {
        return 1;
    }
    return 0;
}

// ══════════════════════════════════════════════════════════════
// Section 2.8: Bitwise compound assignment <<= >>= &= |= ^=
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testBitwiseShiftLeftAssign() {
    let x = 1;
    x <<= 3;
    if (x === 8) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testBitwiseShiftRightAssign() {
    let x = 16;
    x >>= 2;
    if (x === 4) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testBitwiseAndAssign() {
    let x = 15;
    x &= 6;
    if (x === 6) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testBitwiseOrAssign() {
    let x = 5;
    x |= 2;
    if (x === 7) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testBitwiseXorAssign() {
    let x = 12;
    x ^= 10;
    if (x === 6) {
        return 1;
    }
    return 0;
}
