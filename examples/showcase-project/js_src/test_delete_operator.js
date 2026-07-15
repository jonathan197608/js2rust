// BUG-12: `delete obj[key]` on Map generates `deleteByKey(key, alloc)` but
// `alloc` is undeclared in scope. Should use `js_allocator.allocator()`.
// Status: BLOCKED by codegen bug. Enable when BUG-12 is fixed.

/** @returns {i64} */
export function testDeleteMapBracket() {
    const m = new Map();
    m.set("a", 1);
    m.set("b", 2);
    const hadBefore = m.has("a");
    delete m["a"];
    const hasAfter = m.has("a");
    if (hadBefore === true && hasAfter === false) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testDeleteMapComputedKey() {
    const m = new Map();
    m.set("x", 10);
    m.set("y", 20);
    const key = "x";
    delete m[key];
    if (m.has("x") === false && m.has("y") === true) {
        return 1;
    }
    return 0;
}
