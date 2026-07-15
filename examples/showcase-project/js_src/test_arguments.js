// BUG-02: `arguments` object support
// Status: FIXED — arguments.length, arguments[i], and arguments iteration
// all work correctly. Export functions get a __arguments VarDecl with declared
// params (C ABI limitation), non-export functions get a synthetic ...__arguments
// rest param that captures ALL runtime arguments.

// ── arguments.length (export, no params) ──

/** @returns {i64} */
export function testArgumentsLength() {
    return arguments.length;
}

// ── arguments[index] (export, with declared params) ──

/** @returns {i64} */
export function testArgumentsAccess(a, b) {
    const sum = arguments[0] + arguments[1];
    return sum;
}

// ── arguments iteration (export, with declared params) ──

/** @returns {i64} */
export function testArgumentsIterate(a, b, c) {
    let sum = 0;
    for (let i = 0; i < arguments.length; i++) {
        sum = sum + arguments[i];
    }
    return sum;
}

// ── Full variadic support (non-export function with synthetic rest param) ──

function variadicSum() {
    let sum = 0;
    for (let i = 0; i < arguments.length; i++) {
        sum = sum + arguments[i];
    }
    return sum;
}

/** @returns {i64} */
export function testVariadicSum(a, b, c) {
    return variadicSum(a, b, c);
}

function variadicLength() {
    return arguments.length;
}

/** @returns {i64} */
export function testVariadicLength(a, b, c, d) {
    return variadicLength(a, b, c, d);
}
