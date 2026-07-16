// test_for_in_map.js — for...in on Map (iterates keys via .inner.iterator())
// Previously returned @compileError("for-in on this type is not supported").
// Now supported with IrForInKind::MapIter.

/** @returns {i64} */
export function testForInMapCount() {
    const m = new Map();
    m.set("a", 1);
    m.set("b", 2);
    m.set("c", 3);
    let count = 0;
    for (const key in m) {
        if (key === "a" || key === "b" || key === "c") {
            count = count + 1;
        }
    }
    if (count === 3) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testForInMapSingleKey() {
    const m = new Map();
    m.set("x", 42);
    let found = 0;
    for (const key in m) {
        if (key === "x") {
            found = 1;
        }
    }
    return found;
}
