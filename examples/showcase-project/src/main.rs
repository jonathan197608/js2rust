// src/main.rs
// Showcase project for js2rust — JS-to-Zig transpiler
//
// Demonstrates comprehensive transpiler feature coverage:
//   - 3 JS files with 2-level dependency chain (app -> lib -> utils)
//   - Language features: classes, closures, control flow, operators, builtins
//   - C ABI export functions callable from Rust
//
// Note: Only app.js (core file) exports generate C ABI wrappers.
// lib.js and utils.js exports are used internally in the Zig library.

use js2rust_bridge::js2rust_bridge;

// Transpile JS -> Zig and generate FFI bindings.
// Core file: app.js -> imports lib.js -> imports utils.js (2-level dependency)
js2rust_bridge! {
    "js_src/app.js",
}

fn main() {
    // Initialize Zig runtime (allocator for dynamic arrays, strings)
    js2rust_init();

    println!("=== js2rust Showcase Project ===");
    println!("    3 JS files, 2-level dependency chain: app -> lib -> utils");
    println!();

    // ── Basic arithmetic (i64 return) ───────────────────────
    println!("--- Integer functions ---");
    let sum = showcaseSum_app(3, 7);
    println!("  showcaseSum(3, 7) = {}", sum);

    let fact = showcaseFactorial_app(10);
    println!("  showcaseFactorial(10) = {}", fact);

    let prod = showcaseMul_app(6, 7);
    println!("  showcaseMul(6, 7) = {}", prod);

    // ── String functions ────────────────────────────────────
    println!();
    println!("--- String functions ---");
    let greeting = showcaseGreet_app("World");
    println!("  showcaseGreet('World') = {}", greeting);

    let tpl = testTemplate_app(10, 20);
    println!("  testTemplate(10, 20) = {}", tpl);

    // ── Boolean functions ───────────────────────────────────
    println!();
    println!("--- Boolean functions ---");
    let pos5 = showcaseIsPositive_app(5);
    let neg3 = showcaseIsPositive_app(-3);
    println!("  showcaseIsPositive(5) = {}", pos5);
    println!("  showcaseIsPositive(-3) = {}", neg3);

    // ── Control flow ────────────────────────────────────────
    println!();
    println!("--- Control flow ---");
    let mb = testMultiBranch_app(75);
    println!("  testMultiBranch(75) = {}", mb);

    let clamp = testClamp_app(150, 0, 100);
    println!("  testClamp(150, 0, 100) = {}", clamp);

    let abs_neg = testAbsTernary_app(-42);
    println!("  testAbsTernary(-42) = {}", abs_neg);

    let min_val = testMin_app(3, 7);
    println!("  testMin(3, 7) = {}", min_val);

    let max_val = testMax_app(3, 7);
    println!("  testMax(3, 7) = {}", max_val);

    let sign = testSign_app(-5);
    println!("  testSign(-5) = {}", sign);

    // ── Nested function calls ───────────────────────────────
    println!();
    println!("--- Expressions ---");
    let nested = testNestedCalls_app();
    println!("  testNestedCalls() = {} (helper(helper(5)))", nested);

    // ── Full integration test ───────────────────────────────
    println!();
    println!("--- Integration test ---");
    let total = runAllTests_app();
    println!("  runAllTests() = {} (expected 12)", total);

    // Cleanup
    js2rust_deinit();

    println!();
    println!("=== All showcase demos completed ===");
}
