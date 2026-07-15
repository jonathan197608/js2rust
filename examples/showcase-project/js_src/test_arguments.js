// BUG-02: `arguments` object generates ArrayList but .length emits utf16Len
// (wrong function) and [] indexing not supported on ArrayList.
// Status: Partial fix — arguments.length and arguments[i] are correctly
// emitted, but __arguments only contains declared params (not variadic).
// Functions using arguments[index] must declare enough params to avoid
// Zig comptime bounds-check on empty slices.

// ── arguments.length ──

/** @returns {i64} */
export function testArgumentsLength() {
    return arguments.length;
}

/** @returns {i64} */
export function testArgumentsLengthEmpty() {
    return arguments.length;
}

// ── arguments[index] ──

/** @returns {i64} */
export function testArgumentsAccess(a, b) {
    const sum = arguments[0] + arguments[1];
    return sum;
}

// ── arguments iteration ──

/** @returns {i64} */
export function testArgumentsIterate(a, b, c) {
    let sum = 0;
    for (let i = 0; i < arguments.length; i++) {
        sum = sum + arguments[i];
    }
    return sum;
}
