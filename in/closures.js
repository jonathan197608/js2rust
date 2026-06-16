// closures.js — closure tests: var-assigned arrows, callback arrows
// Tests the extended closure detection (non-return arrow functions)

// Named function to test callback pattern
export function applyCallback(x) {
    // This function IS the callback receiver — actual callback
    // will be tested via main.js imports
    return x + 1;
}

// test_* variable for Zig test generation (stripped from Zig output)
const test_applyCallback = applyCallback(1);

// Closure assigned to a variable (captures enclosing scope)
export function createMultiplier(factor) {
    const multiplier = (n) => n * factor;
    return multiplier(5);
}

// Multiple closures in the same function
export function createOperations(base) {
    const add = (x) => x + base;
    const sub = (x) => x - base;
    return add(3) + sub(2);
}
