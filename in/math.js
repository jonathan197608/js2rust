// math.js — arithmetic operations module
// Exports: add, multiply, factorial, clamp, makeAdder

export function add(a, b) {
    return a + b;
}

export function multiply(a, b) {
    return a * b;
}

export function factorial(n) {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

export function clamp(val, min, max) {
    if (val < min) return min;
    if (val > max) return max;
    return val;
}

// Higher-order function returning a closure (arrow function)
export function makeAdder(n) {
    return (x) => x + n;
}
