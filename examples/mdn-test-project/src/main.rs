// src/main.rs — MDN JS Reference tests
// Usage: cargo run 2>&1 | tee actual_output.txt
//        python ../../scripts/compare_output.py expected_output.json actual_output.txt
use js2rust_bridge::js2rust_bridge;

// Transpile JS -> Zig and generate FFI bindings.
// All configuration is read from js2rust.toml.
js2rust_bridge!();

fn main() {
    // Initialize Zig runtime
    js2rust_init();

    println!("=== MDN JS Reference Tests ===");

    // Run expression tests
    println!("\n=== EXPRESSIONS ===");
    let _ = testExpressions_app();

    // Run statement tests
    println!("\n=== STATEMENTS ===");
    let _ = testStatements_app();

    // Run builtin tests
    println!("\n=== BUILTINS ===");
    let _ = testBuiltins_app();

    js2rust_deinit();
    println!("\n=== All tests done ===");
}
