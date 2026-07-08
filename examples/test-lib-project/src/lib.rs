// src/lib.rs
// Demo: use js2rust_bridge!() macro to generate FFI bindings for JS→Zig transpiled code.

use js2rust_bridge::js2rust_bridge;

// Generate FFI bindings: transpiles JS → Zig, generates Rust wrappers.
// All configuration is read from js2rust.toml.
js2rust_bridge!();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        let result = greet("World").unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_add() {
        let result = add(1i64, 2i64);
        assert_eq!(result, 3i64);
    }
}
