// js_src/main.js — Test project for js2rust
// Tests sync and async host functions

/**
 * @param {string} name
 * @returns {string}
 */
export function greet(name) {
    return "Hello, " + name + "!";
}

/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function add(a, b) {
    return a + b;
}

// ── Sync host function examples ──

/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function useHostAdd(a, b) {
    return host_add(a, b);
}

/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function useHostMultiply(a, b) {
    return host_multiply(a, b);
}

/**
 * @param {string} s1
 * @param {string} s2
 * @returns {string}
 */
export function useHostConcat(s1, s2) {
    return host_concat(s1, s2);
}

/**
 * @param {string} s
 * @returns {number}
 */
export function useHostStrlen(s) {
    return host_strlen(s);
}

// ── Async host function example ──

/**
 * @param {string} name
 */
export async function getUserInfo(name) {
    return await fetch_user(name);
}

// ── Try-catch nesting tests ──

/**
 * @returns {number}
 */
export function testNestedTryCatch() {
    let result = 0;
    try {
        try {
            result = 1;
            throw "inner error";
        } catch (e) {
            // Inner catch: should receive "inner error"
            result = 2;
        } finally {
            // Inner finally: should always run
            result = result + 10;
        }
    } catch (e) {
        // Outer catch: should not be reached (inner catch handled the error)
        result = 100;
    } finally {
        // Outer finally: should always run
        result = result + 1000;
    }
    return result; // Expected: 2 + 10 + 1000 = 1012
}

/**
 * @returns {number}
 */
export function testNestedTryCatchWithThrow() {
    let result = 0;
    try {
        try {
            result = 1;
            throw "inner error";
        } catch (e) {
            // Inner catch: handle error, don't re-throw
            result = 2;
        } finally {
            // Inner finally: should always run
            result = result + 10;
        }
    } catch (e) {
        // Outer catch: should not be reached (inner catch handled the error)
        result = 100;
    } finally {
        // Outer finally: should always run
        result = result + 1000;
    }
    return result; // Expected: 2 + 10 + 1000 = 1012
}

/**
 * @returns {number}
 */
export function testTryCatchWithResource() {
    let result = 0;
    try {
        const x = 42;
        try {
            const y = x + 1;
            result = y;
        } catch (e) {
            result = 0;
        }
    } catch (e) {
        result = -1;
    }
    return result; // Expected: 43
}
