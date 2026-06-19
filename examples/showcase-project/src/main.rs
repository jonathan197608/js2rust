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

    // ── Phase 1: Loops ─────────────────────────────────────
    println!();
    println!("--- Loops (Phase 1) ---");
    let for_sum = forSum_app(10);
    println!("  forSum(10) = {} (expected 55)", for_sum);

    let while_halve = whileHalve_app(10);
    println!("  whileHalve(10) = {} (expected 4)", while_halve);

    let do_once = doWhileOnce_app();
    println!("  doWhileOnce() = {} (expected 1)", do_once);

    let for_of = forOfSum_app();
    println!("  forOfSum() = {} (expected 100)", for_of);

    let break5 = breakAtFive_app(100);
    println!("  breakAtFive(100) = {} (expected 15)", break5);

    let cont_even = continueEven_app(10);
    println!("  continueEven(10) = {} (expected 30)", cont_even);

    // ── Phase 2: Error Handling ────────────────────────────
    println!();
    println!("--- Error Handling (Phase 2) ---");
    let tc_basic = tryCatchBasic_app();
    println!("  tryCatchBasic() = {} (expected 42)", tc_basic);

    let tc_side = tryCatchSideEffect_app();
    println!("  tryCatchSideEffect() = {} (expected 15)", tc_side);

    let throw_pos = throwIfNegative_app(5);
    println!("  throwIfNegative(5) = {} (expected 5)", throw_pos);

    let throw_neg = throwIfNegative_app(-3);
    println!("  throwIfNegative(-3) = {} (expected 3)", throw_neg);

    let tc_multi = tryCatchMultiOp_app();
    println!("  tryCatchMultiOp() = {} (expected 30)", tc_multi);

    // ── Phase 3: Operators ─────────────────────────────────
    println!();
    println!("--- Operators (Phase 3) ---");
    let div_val = intDivTest_app();
    println!("  intDivTest() = {} (expected 3)", div_val);

    let mod_val = modOpTest_app();
    println!("  modOpTest() = {} (expected 2)", mod_val);

    let comp = compoundOps_app();
    println!("  compoundOps() = {} (expected 12)", comp);

    let and_tt = logicAnd_app(5, 3);
    println!("  logicAnd(5, 3) = {} (expected 1)", and_tt);

    let and_tf = logicAnd_app(-1, 3);
    println!("  logicAnd(-1, 3) = {} (expected 0)", and_tf);

    let or_tf = logicOr_app(-1, 3);
    println!("  logicOr(-1, 3) = {} (expected 1)", or_tf);

    let or_ff = logicOr_app(-1, -2);
    println!("  logicOr(-1, -2) = {} (expected 0)", or_ff);

    // ── Phase 4: Collections ───────────────────────────────
    println!();
    println!("--- Collections (Phase 4) ---");
    let map_has = testMapHas_app();
    println!("  testMapHas() = {} (expected 1)", map_has);

    let map_miss = testMapMissing_app();
    println!("  testMapMissing() = {} (expected 0)", map_miss);

    let set_has = testSetHas_app();
    println!("  testSetHas() = {} (expected 1)", set_has);

    let set_miss = testSetMissing_app();
    println!("  testSetMissing() = {} (expected 0)", set_miss);

    // Cleanup
    js2rust_deinit();

    println!();
    println!("=== All showcase demos completed ===");
}
