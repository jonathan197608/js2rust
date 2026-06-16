// Try-catch block
function safeDivide(a, b) {
    try {
        if (b === 0) {
            throw "Division by zero";
        }
        return a / b;
    } catch (e) {
        return -1;
    }
}

function useSafeDivide(x, y) {
    const result = safeDivide(x, y);
    return result;
}

// Try-catch-finally with cleanup pattern
// The finally block sets a local variable to track that cleanup ran
function processWithCleanup(a, b) {
    let cleaned = 0;
    try {
        if (b === 0) {
            throw "Division by zero";
        }
        return a / b;
    } catch (e) {
        return -1;
    } finally {
        cleaned = 1;
    }
    return cleaned;
}

export { safeDivide, useSafeDivide, processWithCleanup };

// Expected-value variables for Zig test generation
const test_safeDivide_ok_exceptions = safeDivide(10, 2);
const test_safeDivide_err_exceptions = safeDivide(5, 0);
const test_useSafeDivide_exceptions = useSafeDivide(10, 2);
const test_processWithCleanup_exceptions = processWithCleanup(10, 2);
