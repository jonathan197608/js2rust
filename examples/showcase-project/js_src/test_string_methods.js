// test_string_methods.js
// String method tests on literal strings.
// Previously blocked by BUG-08 (now fixed): string literal type
// handling in codegen was corrected to avoid .deinit()/.items on
// compile-time string types.

/** @returns {i64} */
export function testStringPadStart() {
    const s = "5";
    const padded = s.padStart(3, "0");
    if (padded === "005") {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testStringPadEnd() {
    const s = "abc";
    const padded = s.padEnd(6, ".");
    if (padded === "abc...") {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testStringTrimStart() {
    const s = "  hello  ";
    const trimmed = s.trimStart();
    if (trimmed === "hello  ") {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testStringTrimEnd() {
    const s = "  hello  ";
    const trimmed = s.trimEnd();
    if (trimmed === "  hello") {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testStringSlice() {
    const s = "Hello World";
    const sliced = s.slice(6, 11);
    if (sliced === "World") {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testStringSliceNegative() {
    const s = "Hello World";
    const sliced = s.slice(-5);
    if (sliced === "World") {
        return 1;
    }
    return 0;
}

/** @returns {i64} */
export function testStringSubstring() {
    const s = "Mozilla";
    const sub = s.substring(1, 4);
    if (sub === "ozi") {
        return 1;
    }
    return 0;
}
