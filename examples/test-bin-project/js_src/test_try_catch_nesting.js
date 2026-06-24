// test_try_catch_nesting.js
// Test nested try-catch resource cleanup
//
// Verifies that nested try-catch blocks compile and run correctly.

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

export function testNestedTryCatchWithThrow() {
    let result = 0;
    try {
        try {
            result = 1;
            throw "inner error";
        } catch (e) {
            // Inner catch: re-throw
            result = 2;
            throw "re-thrown error";
        } finally {
            // Inner finally: should always run
            result = result + 10;
        }
    } catch (e) {
        // Outer catch: should receive "re-thrown error"
        result = result + 100;
    } finally {
        // Outer finally: should always run
        result = result + 1000;
    }
    return result; // Expected: 2 + 10 + 100 + 1000 = 1112
}

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
