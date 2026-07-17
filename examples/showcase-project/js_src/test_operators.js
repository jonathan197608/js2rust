// test_operators.js
// End-to-end tests for delete operator.
// Note: `in` operator on Map/Set is tested in test_in_operator.js.

// ── delete operator: use explicit .delete() method on Map ──

/** @returns {i64} */
export function testDeleteMapKey() {
    const m = new Map();
    m.set("a", 1);
    m.set("b", 2);
    const hadBefore = m.has("a");
    m.delete("a");
    const hasAfter = m.has("a");
    if (hadBefore === true && hasAfter === false) {
        return 1;
    }
    return 0;
}

// ── delete on Set ──

/** @returns {i64} */
export function testDeleteSetKey() {
    const s = new Set();
    s.add(10);
    s.add(20);
    const hadBefore = s.has(10);
    s.delete(10);
    const hasAfter = s.has(10);
    if (hadBefore === true && hasAfter === false) {
        return 1;
    }
    return 0;
}
