// phase5.js — Phase 5: Advanced Array Methods
// All advanced array methods: pop, shift, unshift, splice, join, reverse, sort, slice, map, filter

// ── Array.pop ─────────────────────────────────
// arr.pop() → remove and return last element
/**
 * @returns {i64}
 */
export function testArrayPop() {
    const arr = [10, 20, 30];
    arr.pop();
    // Just check that pop() doesn't crash
    return 0;
}

// ── Array.shift ───────────────────────────────────────
// arr.shift() → remove and return first element
/**
 * @returns {i64}
 */
export function testArrayShift() {
    const arr = [10, 20, 30];
    arr.shift();
    // Just check that shift() doesn't crash
    return 0;
}

// ── Array.reverse ─────────────────────────────────────
// arr.reverse() → in-place reverse
/**
 * @returns {i64}
 */
export function testArrayReverse() {
    const arr = [1, 2, 3];
    arr.reverse();
    // Just check that reverse() doesn't crash
    return 0;
}

// ── Array.sort ────────────────────────────────────────
// arr.sort() → in-place sort
/**
 * @returns {i64}
 */
export function testArraySort() {
    const arr = [3, 1, 4, 1, 5, 9, 2, 6];
    arr.sort();
    // Just check that sort() doesn't crash
    return 0;
}

// ── Array.slice ───────────────────────────────────────
// arr.slice(start, end) → return shallow copy
/**
 * @returns {i64}
 */
export function testArraySlice() {
    const arr = [10, 20, 30, 40, 50];
    const sub = arr.slice(1, 4);
    // Check that slice() returns correct length
    if (sub.length === 3) {
        return 0;
    }
    return -1;
}

// DISABLED: delete not yet supported
/*
export function testArraySpliceDelete() {
    const arr = [10, 20, 30, 40, 50];
    arr.splice(1, 2);  // Remove 2 elements at index 1: [10, 40, 50]
    if (arr.length === 3) {
        return 0;
    }
    return -1;
}
*/

// DISABLED: insert not yet supported
/*
export function testArraySpliceInsert() {
    const arr = [10, 20, 30, 40, 50];
    arr.splice(2, 1, 99, 88);  // Remove 1 at index 2, insert 99, 88: [10, 20, 99, 88, 40, 50]
    if (arr.length === 6) {
        return 0;
    }
    return -1;
}
*/

// ── Array.map ───────────────────────────────
// callback inlining: identity function
/**
 * @returns {i64}
 */
export function testArrayMap() {
    const arr = [1, 2, 3, 4, 5];
    const same = arr.map(x => x);  // identity function
    if (same.length === 5) {
        return 0;
    }
    return -1;
}

// ── Array.filter ─────────────────────────────
// callback inlining: keep all
/**
 * @returns {i64}
 */
export function testArrayFilter() {
    const arr = [1, 2, 3, 4, 5];
    const all = arr.filter(x => true);  // keep all
    if (all.length === 5) {
        return 0;
    }
    return -1;
}

// ── Array.reduce ─────────────────────────────
// reduce to sum all elements
/**
 * @returns {i64}
 */
export function testArrayReduce() {
    const arr = [1, 2, 3, 4, 5];
    const sum = arr.reduce((acc, x) => acc + x, 0);
    if (sum === 15) {
        return 0;
    }
    return -1;
}

// ── Array.some ───────────────────────────────
// check if any element > 3
/**
 * @returns {i64}
 */
export function testArraySome() {
    const arr = [1, 2, 3, 4, 5];
    const has_large = arr.some(x => x > 3);
    if (has_large) {
        return 0;
    }
    return -1;
}

// ── Array.every ──────────────────────────────
// check if all elements > 0
/**
 * @returns {i64}
 */
export function testArrayEvery() {
    const arr = [1, 2, 3, 4, 5];
    const all_positive = arr.every(x => x > 0);
    if (all_positive) {
        return 0;
    }
    return -1;
}

// ── Array.forEach (side effects) ────────────────────
// sum all elements by modifying outer variable
/**
 * @returns {i64}
 */
export function testArrayForEach() {
    const arr = [1, 2, 3, 4, 5];
    let sum = 0;
    arr.forEach(x => {
        sum = sum + x;
    });
    if (sum === 15) {
        return 0;
    }
    return -1;
}

// ── Array.some with index ─────────────────────────
// check if any element > 3, using index
/**
 * @returns {i64}
 */
export function testArraySomeIndex() {
    const arr = [1, 2, 3, 4, 5];
    const has_large_idx = arr.some((x, i) => i > 2);
    if (has_large_idx) {
        return 0;
    }
    return -1;
}

// ── Array.every with index ────────────────────────
// check if all elements have index < 5
/**
 * @returns {i64}
 */
export function testArrayEveryIndex() {
    const arr = [10, 20, 30, 40, 50];
    const all_small_idx = arr.every((x, i) => i < 5);
    if (all_small_idx) {
        return 0;
    }
    return -1;
}
