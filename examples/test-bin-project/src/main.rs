// src/main.rs
// Test binary project for js2rust — JS-to-Zig transpiler

use js2rust_bridge::js2rust_bridge;

// Generate FFI bindings for the "main" group
js2rust_bridge!(main);

fn main() {
    // Test greet() JS function
    let result = greet_main("World");
    println!("greet_main('World') = {}", result);

    // Test add() JS function
    let sum = add_main(1, 2);
    println!("add_main(1, 2) = {}", sum);

    // Test multiply() JS function
    let product = multiply_main(3, 4);
    println!("multiply_main(3, 4) = {}", product);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        let result = greet_main("Rust");
        assert_eq!(result, "Hello, Rust!");
    }

    #[test]
    fn test_add() {
        let sum = add_main(10, 20);
        assert_eq!(sum, 30);
    }

    #[test]
    fn test_multiply() {
        let product = multiply_main(6, 7);
        assert_eq!(product, 42);
    }
}
