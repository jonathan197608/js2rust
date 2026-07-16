// test_missing_coverage.js
// E2E tests for expression features previously marked as 隐式测试 or lacking
// dedicated e2e coverage. Each test function returns i64 (1=pass, 0=fail) for
// C ABI compatibility.
//
// Covers:
//   Section 2.1: NaN literal, Infinity literal, undefined (confirm coverage)
//   Section 2.2: -- (decrement), ** (exponent operator)
//   Section 2.3: == (loose equality), != (loose inequality)
//   Section 2.6: void operator
//   Section 2.8: **= (exponent assignment), >>>= (unsigned right shift assign)
//   Section 2.5: >>> (unsigned right shift)
//   Section 2.11: getter in object literal

// ══════════════════════════════════════════════════════════════
// Section 2.1: NaN and Infinity literals
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testNaNLiteral() {
    // NaN is the only value not equal to itself
    const n = NaN;
    if (n !== n) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testInfinityLiteral() {
    const inf = Infinity;
    if (inf > 1000000) { return 1; }
    return 0;
}

// ══════════════════════════════════════════════════════════════
// Section 2.2: Decrement operator --
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testDecrement() {
    let x = 10;
    x--;
    if (x === 9) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testPreDecrement() {
    let x = 5;
    --x;  // x becomes 4
    const y = x;  // y is 4
    if (x === 4 && y === 4) { return 1; }
    return 0;
}

// ══════════════════════════════════════════════════════════════
// Section 2.2: Exponent operator **
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testExponentOp() {
    const result = 2 ** 10;
    if (result === 1024) { return 1; }
    return 0;
}

// ══════════════════════════════════════════════════════════════
// Section 2.8: Exponent assignment **=
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testExponentAssign() {
    let x = 3;
    x **= 2;
    if (x === 9) { return 1; }
    return 0;
}

// ══════════════════════════════════════════════════════════════
// Section 2.8: Unsigned right shift assignment >>>=
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testUnsignedRightShiftAssign() {
    let x = 16;
    x >>>= 2;
    if (x === 4) { return 1; }
    return 0;
}

// ══════════════════════════════════════════════════════════════
// Section 2.5: Unsigned right shift >>>
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testUnsignedRightShift() {
    const result = 16 >>> 2;
    if (result === 4) { return 1; }
    return 0;
}

// ══════════════════════════════════════════════════════════════
// Section 2.6: void operator
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testVoidOperator() {
    // void evaluates its operand then returns undefined
    const result = void 42;
    if (result === undefined) { return 1; }
    return 0;
}

// ══════════════════════════════════════════════════════════════
// Section 2.3: Loose equality == and !=
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testLooseEqual() {
    const a = 42;
    const b = 42;
    if (a == b) { return 1; }
    return 0;
}

/** @returns {i64} */
export function testLooseNotEqual() {
    const a = 1;
    const b = 2;
    if (a != b) { return 1; }
    return 0;
}

// ══════════════════════════════════════════════════════════════
// Section 2.11: Getter in object literal
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testGetterInObjectLiteral() {
    const obj = { get x() { return 42; } };
    return obj.x;
}
