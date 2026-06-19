// js_src/main.js
// Test project for js2rust — JS-to-Zig transpiler

export function greet(name) {
    return "Hello, " + name + "!";
}

export function add(a, b) {
    return a + b;
}

export function multiply(x, y) {
    return x * y;
}

// Host function call examples (commented out — requires type annotations)
// To call Rust host functions from JS:
// 1. Define a JS function that calls the host function
// 2. The transpiler will generate Zig code that calls the host function via C ABI
// Example:
// export function useHostAdd(a, b) {
//     // This would call the Rust hostAdd function
//     // The transpiler needs type annotations to generate correct Zig code
//     return hostAdd(a, b);
// }
