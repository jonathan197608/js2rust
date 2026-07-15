// BUG-01: `in` operator on Map/Set generates `.contains()` which doesn't exist
// on JsCollection. Should generate `.has(JsAny.from(key))` instead.
// Status: BLOCKED by codegen bug. Enable when BUG-01 is fixed.

// ── in operator on Map ──

/** @returns {i64} */
export function testInOperatorMap() {
    const m = new Map();
    m.set("key1", 100);
    m.set("key2", 200);
    if ("key1" in m === true && "missing" in m === false) {
        return 1;
    }
    return 0;
}

// ── in operator on Set ──

/** @returns {i64} */
export function testInOperatorSet() {
    const s = new Set();
    s.add(10);
    s.add(20);
    if (10 in s === true && 99 in s === false) {
        return 1;
    }
    return 0;
}

// ── in operator on dynamic Object (Map-backed) ──

/** @returns {i64} */
export function testInOperatorObj() {
    const obj = new Map();
    obj.set("foo", 1);
    obj.set("bar", 2);
    if ("foo" in obj === true && "baz" in obj === false) {
        return 1;
    }
    return 0;
}
