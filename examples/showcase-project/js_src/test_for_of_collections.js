// test_for_of_collections.js
// for...of iteration on Map, Set, and String, plus Map/Set forEach.
// Previously blocked by BUG-03/04/05/13 (now fixed):
// - BUG-03: Map destructure variables now have correctly inferred types
// - BUG-04: Set iteration variable now has correctly inferred type
// - BUG-05: for-of String unused capture no longer generates error
// - BUG-13: Map/Set.forEach callback params now have correct types

// ── BUG-03: for...of Map with value arithmetic ──

/** @returns {i64} */
export function testForOfMapSumValues() {
    const m = new Map();
    m.set("a", 1);
    m.set("b", 2);
    m.set("c", 3);
    let sum = 0;
    for (const [k, v] of m) {
        sum = sum + v;
    }
    if (sum === 6) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testForOfMapKeyCheck() {
    const m = new Map();
    m.set("x", 10);
    m.set("y", 20);
    let found = 0;
    for (const [k, v] of m) {
        if (k === "x") {
            found = 1;
        }
    }
    if (found === 1) {
        return 1;
    }
    return 0;
}

// ── BUG-04: for...of Set with value arithmetic ──

/** @returns {i64} */
export function testForOfSetSum() {
    const s = new Set();
    s.add(5);
    s.add(10);
    s.add(15);
    let sum = 0;
    for (const val of s) {
        sum = sum + val;
    }
    if (sum === 30) {
        return 1;
    }
    return 0;
}

// ── BUG-05: for...of String with unused capture ──

/** @returns {i64} */
export function testForOfStringCountOnly() {
    let count = 0;
    for (const ch of "ABC") {
        count = count + 1;
    }
    if (count === 3) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testForOfStringByteSum() {
    let sum = 0;
    for (const ch of "ABC") {
        sum = sum + ch;
    }
    // 'A'=65, 'B'=66, 'C'=67 → sum=198
    if (sum === 198) {
        return 1;
    }
    return 0;
}

// ── BUG-13: Map.forEach ──

/** @returns {i64} */
export function testMapForEach() {
    const m = new Map();
    m.set("a", 1);
    m.set("b", 2);
    let sum = 0;
    m.forEach((value, key) => {
        sum = sum + value;
    });
    if (sum === 3) {
        return 1;
    }
    return 0;
}

// ── BUG-13: Set.forEach ──

/** @returns {i64} */
export function testSetForEach() {
    const s = new Set();
    s.add(10);
    s.add(20);
    s.add(30);
    let sum = 0;
    s.forEach((value) => {
        sum = sum + value;
    });
    if (sum === 60) {
        return 1;
    }
    return 0;
}
