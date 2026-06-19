// src/lib.rs
// Demo: use js2rust_bridge!() macro to generate FFI bindings for JS→Zig transpiled code.

// Import the proc-macro directly (bypass js2rust-bridge re-export for now)
use js2rust_bridge_macro::js2rust_bridge;

// Generate FFI bindings for the "main" group (from js_src/main.js)
js2rust_bridge!(main);

// Now you can use the generated safe wrapper functions:
// - greet_main(name: &str) -> String
// - add_main(a: i32, b: i32) -> i32

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        let result = greet_main("World");
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_add() {
        let result = add_main(1i64, 2i64);
        assert_eq!(result, 3i64);
    }
}
