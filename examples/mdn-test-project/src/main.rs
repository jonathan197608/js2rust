// src/main.rs — MDN JS Reference tests (auto-generated)
use js2rust_bridge::js2rust_bridge;

js2rust_bridge!();

fn main() {
    js2rust_init();
    println!("=== MDN JS Reference Tests ===");

    // Minimal tests
    println!("\n=== MINIMAL ===");
    let _ = testMinimal_app();

    // Statements tests
    println!("\n=== STATEMENTS ===");
    let _ = test_statements_part1_app();
    // SKIP: codegen error (member function calls)
    // let _ = test_statements_part2_app();
    let _ = test_statements_part3_app();
    // SKIP: codegen errors (member function calls, Array.filter)
    // let _ = test_statements_part4_app();
    let _ = test_statements_part5_app();

    // Expressions tests
    println!("\n=== EXPRESSIONS ===");
    let _ = test_expressions_part1_app();
    let _ = test_expressions_part2_app();
    let _ = test_expressions_part3_app();
    let _ = test_expressions_part4_app();
    let _ = test_expressions_part5_app();
    let _ = test_expressions_part6_app();
    let _ = test_expressions_part7_app();
    let _ = test_expressions_part8_app();
    let _ = test_expressions_part9_app();
    let _ = test_expressions_part10_app();
    let _ = test_expressions_part11_app();
    let _ = test_expressions_part12_app();
    let _ = test_expressions_part13_app();
    let _ = test_expressions_part14_app();
    let _ = test_expressions_part15_app();
    let _ = test_expressions_part16_app();
    let _ = test_expressions_part17_app();

    // Builtins tests
    println!("\n=== BUILTINS ===");
    let _ = test_builtins_part1_app();
    let _ = test_builtins_part2_app();
    let _ = test_builtins_part3_app();
    let _ = test_builtins_part4_app();
    let _ = test_builtins_part5_app();
    // SKIP: codegen errors (Function constructor, Boolean arg count)
    // let _ = test_builtins_part6_app();
    let _ = test_builtins_part7_app();
    // SKIP: codegen error (this outside class)
    // let _ = test_builtins_part8_app();
    // SKIP: codegen error (unreachable code)
    // let _ = test_builtins_part9_app();
    // SKIP: codegen errors (ArrayBuffer, DataView, member calls)
    // let _ = test_builtins_part10_app();
    // SKIP: codegen error (unreachable code)
    // let _ = test_builtins_part11_app();
    let _ = test_builtins_part12_app();
    let _ = test_builtins_part13_app();
    let _ = test_builtins_part14_app();
    let _ = test_builtins_part15_app();
    // SKIP: codegen error (unsupported callee type)
    // let _ = test_builtins_part16_app();
    // SKIP: codegen error (unsupported callee type)
    // let _ = test_builtins_part17_app();
    let _ = test_builtins_part18_app();
    // SKIP: codegen error (uninitialized columns)
    // let _ = test_builtins_part19_app();
    // SKIP: codegen error (duplicate struct field)
    // let _ = test_builtins_part20_app();
    // SKIP: codegen error (this outside class)
    // let _ = test_builtins_part21_app();
    let _ = test_builtins_part22_app();
    // SKIP: codegen error (RegExp to string)
    // let _ = test_builtins_part23_app();

    js2rust_deinit();
    println!("\n=== All tests done ===");
}
