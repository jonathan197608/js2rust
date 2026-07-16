// test_symbol.js
// Symbol equality, Symbol.for, and Symbol.keyFor tests.
// Previously blocked by BUG-09/10 (now fixed):
// - BUG-09: Symbol equality now uses correct comparison
// - BUG-10: Symbol.keyFor return type handling corrected

// ── BUG-09: Symbol equality ──

/** @returns {i64} */
export function testSymbolUnique() {
    const a = Symbol("test");
    const b = Symbol("test");
    // Each Symbol() is unique
    if (a !== b) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testSymbolFor() {
    const a = Symbol.for("app.key");
    const b = Symbol.for("app.key");
    // Symbol.for returns same symbol for same key
    if (a === b) {
        return 1;
    }
    return 0;
}

// ── BUG-10: Symbol.keyFor returns optional ──

/** @returns {i64} */
export function testSymbolKeyFor() {
    const sym = Symbol.for("my.key");
    const key = Symbol.keyFor(sym);
    if (key === "my.key") {
        return 1;
    }
    return 0;
}
