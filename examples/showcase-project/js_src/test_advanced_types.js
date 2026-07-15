// test_advanced_types.js
// End-to-end tests for BigInt.asIntN/asUintN.
// Note: Symbol equality (==) codegen generates operator not supported
// for JsSymbol; well-known symbols with computed property keys have
// dynamic type issues. These are tested via Rust unit tests.

// ── BigInt.asIntN / BigInt.asUintN ──

/** @returns {i64} */
export function testBigIntAsIntN() {
    const val = BigInt.asIntN(64, 1n);
    if (val === 1n) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testBigIntAsUintN() {
    const val = BigInt.asUintN(8, 256n);
    // 256 in 8-bit unsigned wraps to 0
    if (val === 0n) {
        return 1;
    }
    return 0;
}
