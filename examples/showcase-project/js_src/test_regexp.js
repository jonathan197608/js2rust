// BUG-11: `new RegExp()` generates `try` in a non-error-returning function.
// JsRegExp.init() returns !JsRegExp but the enclosing function is `fn () i64`.
// Status: BLOCKED by codegen bug. Enable when BUG-11 is fixed.

/** @returns {i64} */
export function testRegExpGlobal() {
    const re = new RegExp("test", "g");
    if (re.global === true) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testRegExpIgnoreCase() {
    const re = new RegExp("hello", "i");
    if (re.ignoreCase === true) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testStringSearch() {
    const str = "hello world";
    const idx = str.search(/world/);
    if (idx === 6) {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testStringSearchNotFound() {
    const str = "hello world";
    const idx = str.search(/xyz/);
    if (idx === -1) {
        return 1;
    }
    return 0;
}
