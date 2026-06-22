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
// Multi-root mode: app.js + phase5.js merged into one group.
js2rust_bridge! {
    "js_src/app.js",
    "js_src/phase5.js",
    "js_src/test_throw.js",
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
    let greeting = showcaseGreet_app("World").unwrap();
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
    // Note: try-catch body is not yet implemented (P0-2 pending),
    // so functions with try-catch will propagate throw to Rust as Err.
    println!();
    println!("--- Error Handling (Phase 2) ---");
    let tc_basic = tryCatchBasic_app();
    println!("  tryCatchBasic() = {:?} (expected Err before P0-2)", tc_basic);

    let tc_side = tryCatchSideEffect_app();
    println!("  tryCatchSideEffect() = {:?} (expected Err before P0-2)", tc_side);

    let throw_pos = throwIfNegative_app(5);
    println!("  throwIfNegative(5) = {:?} (expected Err before P0-2)", throw_pos);

    let throw_neg = throwIfNegative_app(-3);
    println!("  throwIfNegative(-3) = {:?} (expected Err)", throw_neg);

    let tc_multi = tryCatchMultiOp_app();
    println!("  tryCatchMultiOp() = {:?} (expected Err before P0-2)", tc_multi);

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

    // ── Phase 5: Map/Set size property ──────────────────────
    println!();
    println!("--- Map/Set size (Phase 5) ---");
    let map_size = testMapSize_app();
    println!("  testMapSize() = {} (expected 1)", map_size);

    let set_size = testSetSize_app();
    println!("  testSetSize() = {} (expected 1)", set_size);

    // ── Phase 5b: Map/Set methods (get/delete) ────────────
    let map_get = testMapGet_app();
    println!("  testMapGet() = {} (expected 1)", map_get);

    let map_del = testMapDelete_app();
    println!("  testMapDelete() = {} (expected 1)", map_del);

    let set_del = testSetDelete_app();
    println!("  testSetDelete() = {} (expected 1)", set_del);

    // ── Phase 5: Array Methods ────────────────────────────
    println!();
    println!("--- Array Methods (Phase 5) ---");

    let pop_val = testArrayPop_app();
    println!("  testArrayPop() = {} (expected 0)", pop_val);

    let shift_val = testArrayShift_app();
    println!("  testArrayShift() = {} (expected 0)", shift_val);

    let rev_ok = testArrayReverse_app();
    println!("  testArrayReverse() = {} (expected 0)", rev_ok);

    let sort_ok = testArraySort_app();
    println!("  testArraySort() = {} (expected 0)", sort_ok);

    let slice_ok = testArraySlice_app();
    println!("  testArraySlice() = {} (expected 0)", slice_ok);

    let map_ok = testArrayMap_app();
    println!("  testArrayMap() = {} (expected 0)", map_ok);

    let filter_ok = testArrayFilter_app();
    println!("  testArrayFilter() = {} (expected 0)", filter_ok);

    let reduce_ok = testArrayReduce_app();
    println!("  testArrayReduce() = {} (expected 0)", reduce_ok);

    let for_each_ok = testArrayForEach_app();
    println!("  testArrayForEach() = {} (expected 0)", for_each_ok);

    let some_idx_ok = testArraySomeIndex_app();
    println!("  testArraySomeIndex() = {} (expected 0)", some_idx_ok);

    let every_idx_ok = testArrayEveryIndex_app();
    println!("  testArrayEveryIndex() = {} (expected 0)", every_idx_ok);

    let some_ok = testArraySome_app();
    println!("  testArraySome() = {} (expected 0)", some_ok);

    let every_ok = testArrayEvery_app();
    println!("  testArrayEvery() = {} (expected 0)", every_ok);

    // ── P0-3: throw→error end-to-end ────────────────────
    println!();
    println!("--- Throw → Error Propagation (P0-3) ---");

    // 1. bareThrowStr: string return with bare throw → Result<String, String>
    let str_ok = bareThrowStr_app(false);
    println!("  bareThrowStr(false) = {:?} (expected Ok(\"ok\"))", str_ok);
    assert!(str_ok.as_deref() == Ok("ok"), "bareThrowStr(false) should return Ok(\"ok\"), got {:?}", str_ok);

    let str_err = bareThrowStr_app(true);
    println!("  bareThrowStr(true) = {:?} (expected Err)", str_err);
    assert!(str_err.is_err(), "bareThrowStr(true) should be Err, got {:?}", str_err);

    // 2. bareThrowI64: i64 return with bare throw → Result<i64, String>
    let i64_ok = bareThrowI64_app(false);
    println!("  bareThrowI64(false) = {:?} (expected Ok(42))", i64_ok);
    assert_eq!(i64_ok, Ok(42), "bareThrowI64(false) should be Ok(42)");

    let i64_err = bareThrowI64_app(true);
    println!("  bareThrowI64(true) = {:?} (expected Err)", i64_err);
    assert!(i64_err.is_err(), "bareThrowI64(true) should be Err, got {:?}", i64_err);

    // 3. bareThrowVoid: void return with bare throw → Result<(), String>
    let void_ok = bareThrowVoid_app(false);
    println!("  bareThrowVoid(false) = {:?} (expected Ok(()))", void_ok);
    assert_eq!(void_ok, Ok(()), "bareThrowVoid(false) should be Ok(())");

    let void_err = bareThrowVoid_app(true);
    println!("  bareThrowVoid(true) = {:?} (expected Err)", void_err);
    assert!(void_err.is_err(), "bareThrowVoid(true) should be Err, got {:?}", void_err);

    // 4. caughtThrow: try-catch catches → normal return (no error)
    let caught_ok = caughtThrow_app(false);
    println!("  caughtThrow(false) = {:?} (expected Ok(100))", caught_ok);
    assert_eq!(caught_ok, Ok(100), "caughtThrow(false) should be Ok(100)");

    let caught_err = caughtThrow_app(true);
    println!("  caughtThrow(true) = {:?} (expected Ok(-1), error caught)", caught_err);
    assert_eq!(caught_err, Ok(-1), "caughtThrow(true) should be Ok(-1) (error caught internally)");

    // 5. tryFinally: try-catch-finally → finally always runs
    let tf_ok = tryFinally_app(false);
    println!("  tryFinally(false) = {:?} (expected Ok(11))", tf_ok);
    assert_eq!(tf_ok, Ok(11), "tryFinally(false): 10 + 1 from finally = 11");

    let tf_err = tryFinally_app(true);
    println!("  tryFinally(true) = {:?} (expected Ok(-9))", tf_err);
    assert_eq!(tf_err, Ok(-9), "tryFinally(true): -10 + 1 from finally = -9");

    println!("  All throw→error tests passed!");

    // Cleanup
    js2rust_deinit();

    println!();
    println!("=== All showcase demos completed ===");
}
