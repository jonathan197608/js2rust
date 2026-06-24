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

/**
 * Test re-throw from inner catch to outer catch.
 * Verifies that `throw` in catch body propagates to the enclosing try-catch
 * rather than returning from the whole function.
 * @returns {number}
 */
export function testNestedTryCatchReThrow() {
    let result = 0;
    try {
        try {
            result = 1;
            throw "inner error";
        } catch (e) {
            // Inner catch: re-throw
            result = 2;
            throw "re-thrown to outer";
        } finally {
            // Inner finally: should always run
            result = result + 10;
        }
    } catch (e) {
        // Outer catch: should receive "re-thrown to outer"
        result = result + 100;
    } finally {
        // Outer finally: should always run
        result = result + 1000;
    }
    return result; // Expected: 2 + 10 + 100 + 1000 = 1112
}

// ── Date tests ──

/**
 * new Date() — current timestamp
 * @returns {number}
 */
export function testNewDate() {
    const d = new Date();
    return d.getTime();
}

/**
 * new Date(millis) — from specific timestamp
 * @returns {number}
 */
export function testNewDateWithMillis() {
    const d = new Date(1000);
    return d.getTime();
}

/**
 * Date.getFullYear() — year extraction
 * @returns {number}
 */
export function testDateGetFullYear() {
    const d = new Date(0);
    return d.getFullYear();
}

/**
 * Date.getDay() — day of week (0=Sun, epoch=Thursday=4)
 * @returns {number}
 */
export function testDateGetDay() {
    const d = new Date(0);
    return d.getDay();
}

/**
 * Date.getHours() — hours from epoch (0 UTC)
 * @returns {number}
 */
export function testDateGetHours() {
    const d = new Date(0);
    return d.getHours();
}

/**
 * Date.getMonth() — month (0-based, epoch=January=0)
 * @returns {number}
 */
export function testDateGetMonth() {
    const d = new Date(0);
    return d.getMonth();
}

/**
 * Date.getDate() — day of month (epoch=1)
 * @returns {number}
 */
export function testDateGetDate() {
    const d = new Date(0);
    return d.getDate();
}

/**
 * Date.getMinutes() and getSeconds() — from epoch=0 UTC
 * @returns {number}
 */
export function testDateGetMinutes() {
    const d = new Date(0);
    return d.getMinutes();
}

/**
 * @returns {number}
 */
export function testDateGetSeconds() {
    const d = new Date(0);
    return d.getSeconds();
}
