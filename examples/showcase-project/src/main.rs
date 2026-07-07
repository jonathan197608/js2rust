// src/main.rs — minimal showcase for testing js2rust features
use js2rust_bridge::js2rust_bridge;

// Transpile JS -> Zig and generate FFI bindings.
// All configuration is read from js2rust.toml.
js2rust_bridge!();

fn main() {
    // Initialize Zig runtime
    js2rust_init();

    println!("=== js2rust Showcase (P0/P1 test) ===");

    // Phase 0: Basic arithmetic (i64 return)
    let sum = showcaseSum_app(3, 7);
    println!("  showcaseSum(3,7) = {} (expected 10)", sum);

    // Phase 5: Array methods
    let pop_ok = testArrayPop_app();
    println!("  testArrayPop() = {} (expected 0)", pop_ok);

    let reduce_ok = testArrayReduce_app();
    println!("  testArrayReduce() = {} (expected 0)", reduce_ok);

    let for_each_ok = testArrayForEach_app();
    println!("  testArrayForEach() = {} (expected 0)", for_each_ok);

    let map_ok = testArrayMap_app();
    println!("  testArrayMap() = {} (expected 0)", map_ok);

    let filter_ok = testArrayFilter_app();
    println!("  testArrayFilter() = {} (expected 0)", filter_ok);

    let some_ok = testArraySome_app();
    println!("  testArraySome() = {} (expected 0)", some_ok);

    let every_ok = testArrayEvery_app();
    println!("  testArrayEvery() = {} (expected 0)", every_ok);

    // Phase 2: Throw -> Error propagation
    let caught_ok = caughtThrow_app(false);
    println!("  caughtThrow(false) = {:?}", caught_ok);

    let tf = tryFinally_app(false);
    println!("  tryFinally(false) = {:?}", tf);

    // ── Memory: dual-arena stress tests ───────────────────
    println!("\n--- Memory Stress Tests ---");

    let map_ok = testMapStress_app();
    println!("  testMapStress() = {} (expected 1)", map_ok);

    let set_ok = testSetStress_app();
    println!("  testSetStress() = {} (expected 1)", set_ok);

    let arr_mut = testArrayMutStress_app();
    println!("  testArrayMutStress() = {} (expected 0)", arr_mut);

    let add_ok = testMemoryAdd_app(2, 3);
    println!("  testMemoryAdd(2,3) = {} (expected 5)", add_ok);

    // MultiArenaAllocator auto-manages arena rotation (cooling + reset)
    // No manual reset needed — the old js2rust_reset() was a no-op.

    // Verify correctness after rotation
    let post_reduce = testArrayReduce_app();
    println!(
        "  testArrayReduce (after rotation) = {} (expected 0)",
        post_reduce
    );

    let post_add = testMemoryAdd_app(7, 8);
    println!(
        "  testMemoryAdd(7,8) after reset = {} (expected 15)",
        post_add
    );

    let post_greeting = testLongGreeting_app("World");
    println!("  testLongGreeting('World') = {:?}", post_greeting);

    // ════════════════════════════════════════════════════════════
    // Phase 6: Built-in Objects Deep Dive
    // String, Math, Date, Number, Object methods
    println!("\n=== Phase 6: Built-in Objects ===");

    // ── String methods ──
    let s_idx = testStringIndexOf_app();
    println!("  testStringIndexOf() = {} (expected 0)", s_idx);

    let s_nf = testStringIndexOfNotFound_app();
    println!("  testStringIndexOfNotFound() = {} (expected 0)", s_nf);

    let s_inc = testStringIncludes_app();
    println!("  testStringIncludes() = {} (expected 0)", s_inc);

    let s_inc_nf = testStringIncludesNotFound_app();
    println!("  testStringIncludesNotFound() = {} (expected 0)", s_inc_nf);

    let s_sw = testStringStartsWith_app();
    println!("  testStringStartsWith() = {} (expected 0)", s_sw);

    let s_sw_f = testStringStartsWithFalse_app();
    println!("  testStringStartsWithFalse() = {} (expected 0)", s_sw_f);

    let s_ew = testStringEndsWith_app();
    println!("  testStringEndsWith() = {} (expected 0)", s_ew);

    let s_ew_f = testStringEndsWithFalse_app();
    println!("  testStringEndsWithFalse() = {} (expected 0)", s_ew_f);

    let s_trim = testStringTrim_app();
    println!("  testStringTrim() = {} (expected 0)", s_trim);

    // ── Math methods ──
    let m_abs = testMathAbs_app();
    println!("  testMathAbs() = {} (expected 0)", m_abs);

    let m_floor = testMathFloor_app();
    println!("  testMathFloor() = {} (expected 0)", m_floor);

    let m_ceil = testMathCeil_app();
    println!("  testMathCeil() = {} (expected 0)", m_ceil);

    let m_round = testMathRound_app();
    println!("  testMathRound() = {} (expected 0)", m_round);

    let m_max = testMathMax_app();
    println!("  testMathMax() = {} (expected 0)", m_max);

    let m_min = testMathMin_app();
    println!("  testMathMin() = {} (expected 0)", m_min);

    // ── Date methods ──
    let d_now = testDateNow_app();
    println!("  testDateNow() = {} (expected 0)", d_now);

    let d_from = testDateFromMillis_app();
    println!("  testDateFromMillis() = {} (expected 0)", d_from);

    let d_year = testDateGetFullYear_app();
    println!("  testDateGetFullYear() = {} (expected 0)", d_year);

    let d_month = testDateGetMonth_app();
    println!("  testDateGetMonth() = {} (expected 0)", d_month);

    let d_date = testDateGetDate_app();
    println!("  testDateGetDate() = {} (expected 0)", d_date);

    let d_day = testDateGetDay_app();
    println!("  testDateGetDay() = {} (expected 0)", d_day);

    let d_hours = testDateGetHours_app();
    println!("  testDateGetHours() = {} (expected 0)", d_hours);

    let d_now_s = testDateNowStatic_app();
    println!("  testDateNowStatic() = {} (expected 0)", d_now_s);

    let d_min = testDateGetMinutes_app();
    println!("  testDateGetMinutes() = {} (expected 0)", d_min);

    let d_sec = testDateGetSeconds_app();
    println!("  testDateGetSeconds() = {} (expected 0)", d_sec);

    let d_min_e = testDateMinutesEpoch_app();
    println!("  testDateMinutesEpoch() = {} (expected 0)", d_min_e);

    let d_sec_e = testDateSecondsEpoch_app();
    println!("  testDateSecondsEpoch() = {} (expected 0)", d_sec_e);

    let d_comp = testDateComposite_app();
    println!("  testDateComposite() = {} (expected 0)", d_comp);

    let d_day_v = testDateGetDayVerify_app();
    println!("  testDateGetDayVerify() = {} (expected 0)", d_day_v);

    // ── Number methods ──
    let pi = testParseInt_app();
    println!("  testParseInt() = {} (expected 0)", pi);

    let pi_hex = testParseIntHex_app();
    println!("  testParseIntHex() = {} (expected 0)", pi_hex);

    // ── Object methods ──
    let ok = testObjectKeys_app();
    println!("  testObjectKeys() = {} (expected 0)", ok);

    // ════════════════════════════════════════════════════════════
    // Phase 6 extra: Object spread merge
    println!("\n=== Spread Merge Tests ===");

    let ss = testSpreadSingle_app();
    println!("  testSpreadSingle() = {} (expected 0)", ss);

    let sm = testSpreadMulti_app();
    println!("  testSpreadMulti() = {} (expected 0)", sm);

    let st = testSpreadTriple_app();
    println!("  testSpreadTriple() = {} (expected 0)", st);

    let swi = testSpreadWithInline_app();
    println!("  testSpreadWithInline() = {} (expected 0)", swi);

    let so = testSpreadOverride_app();
    println!("  testSpreadOverride() = {} (expected 0)", so);

    // ════════════════════════════════════════════════════════════
    // Type inference verification: known limitations check
    println!("\n=== Type Inference Verification ===");

    // -- Division / Modulo (integer: @divTrunc / @rem) --
    let div = intDivTest_app();
    println!("  intDivTest(17/5 via assign) = {} (expected 3)", div);

    let mod_op = modOpTest_app();
    println!("  modOpTest(17%5 via assign) = {} (expected 2)", mod_op);

    let div_expr = testDivExpr_app();
    println!("  testDivExpr(17/5 expr) = {} (expected 3)", div_expr);

    let mod_expr = testModExpr_app();
    println!("  testModExpr(17%5 expr) = {} (expected 2)", mod_expr);

    // -- Bitwise & | ^ --
    let bw_and = testBitwiseAnd_app();
    println!("  testBitwiseAnd(12&10) = {} (expected 1)", bw_and);

    let bw_or = testBitwiseOr_app();
    println!("  testBitwiseOr(12|10) = {} (expected 1)", bw_or);

    let bw_xor = testBitwiseXor_app();
    println!("  testBitwiseXor(12^10) = {} (expected 1)", bw_xor);

    // -- Map.delete() / Set.delete() return value --
    let md = testMapDelete_app();
    println!("  testMapDelete() = {} (expected 1)", md);

    let sd = testSetDelete_app();
    println!("  testSetDelete() = {} (expected 1)", sd);

    // ========== P2: Destructuring Defaults ==========
    let dod = testDestructureObjDefault_app();
    println!("  testDestructureObjDefault() = {} (expected 1)", dod);

    let doe = testDestructureObjDefaultEmpty_app();
    println!("  testDestructureObjDefaultEmpty() = {} (expected 1)", doe);

    let dad = testDestructureArrDefault_app();
    println!("  testDestructureArrDefault() = {} (expected 1)", dad);

    let dae = testDestructureArrDefaultEmpty_app();
    println!("  testDestructureArrDefaultEmpty() = {} (expected 1)", dae);

    // ════════════════════════════════════════════════════════════
    // Class support tests
    println!("\n=== Class Support Tests ===");

    let ra = testRectArea_app();
    println!("  testRectArea() = {} (expected 12)", ra);

    let rp = testRectPerim_app();
    println!("  testRectPerim() = {} (expected 60)", rp);

    let uid = testUserId_app();
    println!("  testUserId() = {} (expected 42)", uid);

    let unl = testUserNameLength_app();
    println!("  testUserNameLength() = {} (expected 1)", unl);

    // ════════════════════════════════════════════════════════════
    // Static field tests (static block runtime needs bridge init mechanism)
    println!("\n=== Static Field Tests ===");

    let sfr = testStaticFieldRead_app();
    println!("  testStaticFieldRead() = {} (expected 0)", sfr);

    let sfa = testStaticFieldAssign_app();
    println!("  testStaticFieldAssign() = {} (expected 0)", sfa);

    let sfm = testStaticFieldMultiply_app();
    println!("  testStaticFieldMultiply() = {} (expected 0)", sfm);

    let sfsg = testStaticFieldSetThenGet_app();
    println!("  testStaticFieldSetThenGet() = {} (expected 42)", sfsg);

    // ════════════════════════════════════════════════════════════
    // Dynamic array/string index tests
    println!("\n=== Dynamic Index Tests ===");

    let daa = testDynamicArrayAccess_app(2);
    println!("  testDynamicArrayAccess(2) = {} (expected 30)", daa);

    let das = testDynamicArrayAssign_app(1, 99);
    println!("  testDynamicArrayAssign(1,99) = {} (expected 99)", das);

    let dsum = testDynamicArraySum_app();
    println!("  testDynamicArraySum() = {} (expected 15)", dsum);

    let dsi = testDynamicStringIndex_app("Hello World", 0);
    println!("  testDynamicStringIndex('Hello World',0) = {} (expected 1, checks byte 72='H')", dsi);

    let dsw = testDynamicArraySwap_app(0, 2);
    println!("  testDynamicArraySwap(0,2) = {} (expected 300)", dsw);

    js2rust_deinit();
    println!("=== All tests done ===");
}
