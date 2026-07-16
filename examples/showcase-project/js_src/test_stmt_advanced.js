// test_stmt_advanced.js
// End-to-end tests for statement features: labeled for-of, nested try-catch.
//
// NOTE: Rest parameters ...args cannot be tested in e2e:
//   - rest params generate `[]const JsAny` (Anytype), not C-safe for export
//   - Non-export wrapper still can't spread args: `restSum(1,2,3,4,5)` becomes
//     a single-arg call with the rest array, not a spread — codegen limitation.
//   Covered by Rust unit tests only.

// ══════════════════════════════════════════════════════════════
// Section 3.4: Labeled for-of with break
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testLabeledForOf() {
    const arr = [10, 20, 30];
    let sum = 0;
    outer: for (const v of arr) {
        sum = sum + v;
        if (v === 20) {
            break outer;
        }
    }
    if (sum === 30) {
        return 1;
    }
    return 0;
}

// ══════════════════════════════════════════════════════════════
// Section 3.5: Nested try-catch
// ══════════════════════════════════════════════════════════════

/** @returns {i64} */
export function testNestedTryCatch() {
    let result = 0;
    try {
        try {
            throw "inner";
        } catch (e) {
            result = 1;
        }
        result = result + 10;
    } catch (e2) {
        result = -1;
    }
    if (result === 11) {
        return 1;
    }
    return 0;
}
