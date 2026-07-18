// test_dynamic_array_index.js — Variable array index & dynamic string index tests
// Covers: arr[idx] read, arr[idx] = val write, str.charCodeAt(idx) with JSDoc

// ── Variable array index: read ────────────────────────────────

/**
 * @param {i64} idx
 * @returns {i64}
 */
export function testDynamicArrayAccess(idx) {
    const arr = [10, 20, 30, 40, 50];
    const val = arr[idx];
    return val;
}

// ── Variable array index: write ───────────────────────────────

/**
 * @param {i64} idx
 * @param {i64} val
 * @returns {i64}
 */
export function testDynamicArrayAssign(idx, val) {
    const arr = [10, 20, 30, 40, 50];
    arr[idx] = val;
    return arr[idx];
}

// ── Loop-based dynamic index: sum ─────────────────────────────

/** @returns {i64} */
export function testDynamicArraySum() {
    const arr = [1, 2, 3, 4, 5];
    let sum = 0;
    for (let i = 0; i < 5; i++) {
        sum = sum + arr[i];
    }
    return sum;
}

// ── Dynamic string index: charCodeAt via JSDoc annotation ──────
// Note: str[idx] in JS returns a single-character substring ([]const u8 in
// Zig). To compare against an ASCII code numerically, use String.prototype
// .charCodeAt() which yields a UTF-16 code unit (u16, widened to i64 here).

/**
 * @param {string} s
 * @param {i64} idx
 * @returns {i64}
 */
export function testDynamicStringIndex(s, idx) {
    const ch = s.charCodeAt(idx);
    // 'H' = 72, 'W' = 87
    if (ch === 72) { return 1; }
    if (ch === 87) { return 2; }
    return 0;
}

// ── Swap via dynamic index ────────────────────────────────────

/**
 * @param {i64} i
 * @param {i64} j
 * @returns {i64}
 */
export function testDynamicArraySwap(i, j) {
    const arr = [100, 200, 300];
    const tmp = arr[i];
    arr[i] = arr[j];
    arr[j] = tmp;
    return arr[i];
}
