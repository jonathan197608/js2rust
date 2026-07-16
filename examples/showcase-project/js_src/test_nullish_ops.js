// test_nullish_ops.js
// End-to-end tests for nullish coalescing operator ??.
// Isolated in a separate file because null/undefined operands cause
// the transpiler to infer JsAny types, which would pollute other tests.
//
// NOTE: ??= / &&= / ||= cannot be tested in e2e:
//   - ??= on null generates `x = if (x.isNullish()) 99 else x` where 99 is
//     comptime_int, incompatible with JsAny — codegen limitation.
//   - &&= / ||= on i64 generate `.toBool()` which doesn't exist on i64.
//   These are covered by Rust unit tests only.

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
