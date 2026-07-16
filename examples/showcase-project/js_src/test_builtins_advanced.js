// test_builtins_advanced.js
// End-to-end tests for advanced built-in methods.
// Previously noted codegen issues with Date setters, String methods on
// literals, Map/Set forEach, and RegExp init — all have been fixed.
// Comprehensive coverage of these methods is in test_builtins_coverage.js.

// ── Date UTC getter methods (work correctly) ──

/** @returns {i64} */
export function testDateGetUTCFullYear() {
    // 2020-01-01T00:00:00Z = 1577836800000
    const d = new Date(1577836800000);
    const year = d.getUTCFullYear();
    if (year === 2020) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testDateGetUTCMonth() {
    const d = new Date(1577836800000);
    const month = d.getUTCMonth();
    if (month === 0) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testDateGetUTCDate() {
    const d = new Date(1577836800000);
    const date = d.getUTCDate();
    if (date === 1) {
        return 1;
    }
    return 0;
}

// ── Number static methods (i64 return, no type issues) ──

/** @returns {i64} */
export function testNumberIsFinite() {
    const a = Number.isFinite(42);
    if (a === true) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testNumberIsInteger() {
    const a = Number.isInteger(7);
    const b = Number.isInteger(7.5);
    if (a === true && b === false) {
        return 1;
    }
    return 0;
}

// ── String slice on variable (not literal) ──

/** @param {string} s
    @returns {string} */
export function testStringSliceParam(s) {
    const sliced = s.slice(0, 5);
    return sliced;
}

/** @param {string} s
    @returns {string} */
export function testStringSubstringParam(s) {
    const sub = s.substring(0, 3);
    return sub;
}
