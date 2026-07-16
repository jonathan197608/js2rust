// test_regexp.js
// RegExp constructor and instance method tests.
// Previously blocked by BUG-11 (now fixed): RegExp init no longer
// generates try in a non-error-returning function.

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
