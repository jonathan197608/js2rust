// BUG-02: `arguments` object generates ArrayList but .length emits utf16Len
// (wrong function) and [] indexing not supported on ArrayList.
// Status: BLOCKED by codegen bug. Enable when BUG-02 is fixed.

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
export function testArgumentsAccess() {
    const sum = arguments[0] + arguments[1];
    return sum;
}

// ── arguments iteration ──

/** @returns {i64} */
export function testArgumentsIterate() {
    let sum = 0;
    for (let i = 0; i < arguments.length; i++) {
        sum = sum + arguments[i];
    }
    return sum;
}
