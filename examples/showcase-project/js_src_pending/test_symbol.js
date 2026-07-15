// BUG-09/10: Symbol equality generates == / != on JsSymbol struct which
// doesn't support those operators. Symbol.keyFor() returns ?[]const u8
// but code treats it as []const u8.
// Also: well-known symbols used as computed property keys generate
// dynamic object type .{ } which is invalid.
// Status: BLOCKED by codegen bugs. Enable when BUG-09/10 are fixed.

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

// ── Well-known symbols as computed property keys ──

/** @returns {i64} */
export function testWellKnownSymbolIterator() {
    const obj = {};
    obj[Symbol.iterator] = 42;
    const val = obj[Symbol.iterator];
    if (val === 42) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testWellKnownSymbolToStringTag() {
    const obj = {};
    obj[Symbol.toStringTag] = "MyType";
    const val = obj[Symbol.toStringTag];
    if (val === "MyType") {
        return 1;
    }
    return 0;
}
