// test_throw.js
// End-to-end tests for throw → C ABI error propagation.
// Tests: StrRet.from_panic (string), err_out (i64/void), try-catch cleanup.

// ── Bare throw with string return → StrRet.from_panic ──
/** @param {boolean} shouldThrow
    @returns {string} */
export function bareThrowStr(shouldThrow) {
    if (shouldThrow) {
        throw "string error";
    }
    return "ok";
}

// ── Bare throw with i64 return → err_out → Result<i64, String> ──
/** @param {boolean} shouldThrow
    @returns {i64} */
export function bareThrowI64(shouldThrow) {
    if (shouldThrow) {
        throw "i64 error";
    }
    return 42;
}

// ── Bare throw with void return → err_out → Result<(), String> ──
/** @param {boolean} shouldThrow
    @returns {void} */
export function bareThrowVoid(shouldThrow) {
    if (shouldThrow) {
        throw "void error";
    }
}

// ── try-catch: error caught internally → normal return ──
/** @param {boolean} shouldThrow
    @returns {i64} */
export function caughtThrow(shouldThrow) {
    try {
        if (shouldThrow) {
            throw "caught";
        }
        return 100;
    } catch (e) {
        return -1;
    }
}

// ── try-catch-finally: finally runs regardless ──
/** @param {boolean} triggerThrow
    @returns {i64} */
export function tryFinally(triggerThrow) {
    let result = 0;
    try {
        if (triggerThrow) {
            throw "finally test";
        }
        result = 10;
    } catch (e) {
        result = -10;
    } finally {
        result = result + 1;
    }
    return result;
}
