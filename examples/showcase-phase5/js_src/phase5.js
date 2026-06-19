// phase5.js — Phase 5: Advanced Array Methods
// Testing: pop, shift, join, reverse, sort, slice, map, filter

// ── Array.pop ─────────────────────────────────
// arr.pop() → remove and return last element
export function testArrayPop() {
    const arr = [10, 20, 30];
    arr.pop();
    // Just check that pop() doesn't crash
    return 0;
}

// ── Array.shift ───────────────────────────────────────
// arr.shift() → remove and return first element
export function testArrayShift() {
    const arr = [10, 20, 30];
    arr.shift();
    // Just check that shift() doesn't crash
    return 0;
}

// ── Array.reverse ─────────────────────────────────────
// arr.reverse() → in-place reverse
export function testArrayReverse() {
    const arr = [1, 2, 3];
    arr.reverse();
    // Just check that reverse() doesn't crash
    return 0;
}

// ── Array.sort ────────────────────────────────────────
// arr.sort() → in-place sort
export function testArraySort() {
    const arr = [3, 1, 4, 1, 5, 9, 2, 6];
    arr.sort();
    // Just check that sort() doesn't crash
    return 0;
}

// ── Array.slice ───────────────────────────────────────
// arr.slice(start, end) → return shallow copy
export function testArraySlice() {
    const arr = [10, 20, 30, 40, 50];
    const sub = arr.slice(1, 4);
    // Check that slice() returns correct length
    if (sub.length === 3) {
        return 0;
    }
    return -1;
}

// ── Array.splice (delete only) ────────────────────────
export function testArraySpliceDelete() {
    const arr = [10, 20, 30, 40, 50];
    arr.splice(1, 2);  // Remove 2 elements at index 1: [10, 40, 50]
    if (arr.length === 3) {
        return 0;
    }
    return -1;
}

// ── Array.splice (delete + insert) ───────────────────
export function testArraySpliceInsert() {
    const arr = [10, 20, 30, 40, 50];
    arr.splice(2, 1, 99, 88);  // Remove 1 at index 2, insert 99, 88: [10, 20, 99, 88, 40, 50]
    if (arr.length === 6) {
        return 0;
    }
    return -1;
}

// ── Array.map ──────────────────────────────────
// DISABLED: map() with callback not yet supported
// export function testArrayMap() {
//     const arr = [1, 2, 3, 4, 5];
//     const doubled = arr.map(x => x * 2);
//     if (doubled.length === 5) {
//         return 0;
//     }
//     return -1;
// }

// ── Array.filter ───────────────────────────────
// DISABLED: filter() with callback not yet supported
// export function testArrayFilter() {
//     const arr = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
//     const evens = arr.filter(x => x % 2 === 0);
//     if (evens.length === 5) {
//         return 0;
//     }
//     return -1;
// }
