// test_nullish_ops.js
// End-to-end tests for nullish coalescing operator ?? and ??=.
// Isolated in a separate file because null/undefined operands cause
// the transpiler to infer JsAny types, which would pollute other tests.

// ══════════════════════════════════════════════════════════════
// Section 2.4: Nullish coalescing operator ??
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testNullishCoalescing() {
    const val = null ?? 42;
    if (val === 42) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testNullishCoalescingUndefined() {
    const val = undefined ?? 7;
    if (val === 7) {
        return 1;
    }
    return 0;
}

// ── ??= (nullish assignment) ──

/** @returns {i64} */
export function testNullishAssignNull() {
    let val = null;
    val ??= 42;  // val is nullish, so val = 42
    if (val === 42) {
        return 1;
    }
    return 0;
}
