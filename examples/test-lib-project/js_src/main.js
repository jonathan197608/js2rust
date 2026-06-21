// js_src/main.js
// Simple JS function to demo JS→Zig→Rust FFI

/**
 * @param {string} name
 * @returns {string}
 */
export function greet(name) {
    return "Hello, " + name + "!";
}

/**
 * @param {i64} a
 * @param {i64} b
 * @returns {i64}
 */
export function add(a, b) {
    return a + b;
}
