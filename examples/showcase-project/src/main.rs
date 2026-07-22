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
    let sum = showcaseSum(3, 7);
    println!("  showcaseSum(3,7) = {} (expected 10)", sum);

    // Phase 5: Array methods
    let pop_ok = testArrayPop();
    println!("  testArrayPop() = {} (expected 0)", pop_ok);

    let reduce_ok = testArrayReduce();
    println!("  testArrayReduce() = {} (expected 0)", reduce_ok);

    let for_each_ok = testArrayForEach();
    println!("  testArrayForEach() = {} (expected 0)", for_each_ok);

    let map_ok = testArrayMap();
    println!("  testArrayMap() = {} (expected 0)", map_ok);

    let filter_ok = testArrayFilter();
    println!("  testArrayFilter() = {} (expected 0)", filter_ok);

    let some_ok = testArraySome();
    println!("  testArraySome() = {} (expected 0)", some_ok);

    let every_ok = testArrayEvery();
    println!("  testArrayEvery() = {} (expected 0)", every_ok);

    // Phase 2: Throw -> Error propagation
    let caught_ok = caughtThrow(false);
    println!("  caughtThrow(false) = {:?}", caught_ok);

    let tf = tryFinally(false);
    println!("  tryFinally(false) = {:?}", tf);

    // ── Memory: dual-arena stress tests ───────────────────
    println!("\n--- Memory Stress Tests ---");

    let map_ok = testMapStress();
    println!("  testMapStress() = {} (expected 1)", map_ok);

    let set_ok = testSetStress();
    println!("  testSetStress() = {} (expected 1)", set_ok);

    let arr_mut = testArrayMutStress();
    println!("  testArrayMutStress() = {} (expected 0)", arr_mut);

    let add_ok = testMemoryAdd(2, 3);
    println!("  testMemoryAdd(2,3) = {} (expected 5)", add_ok);

    // MultiArenaAllocator auto-manages arena rotation (cooling + reset)
    // No manual reset needed — the old js2rust_reset() was a no-op.

    // Verify correctness after rotation
    let post_reduce = testArrayReduce();
    println!(
        "  testArrayReduce (after rotation) = {} (expected 0)",
        post_reduce
    );

    let post_add = testMemoryAdd(7, 8);
    println!(
        "  testMemoryAdd(7,8) after reset = {} (expected 15)",
        post_add
    );

    let post_greeting = testLongGreeting("World");
    println!("  testLongGreeting('World') = {:?}", post_greeting);

    // ════════════════════════════════════════════════════════════
    // Phase 6: Built-in Objects Deep Dive
    // String, Math, Date, Number, Object methods
    println!("\n=== Phase 6: Built-in Objects ===");

    // ── String methods ──
    let s_idx = testStringIndexOf();
    println!("  testStringIndexOf() = {} (expected 0)", s_idx);

    let s_nf = testStringIndexOfNotFound();
    println!("  testStringIndexOfNotFound() = {} (expected 0)", s_nf);

    let s_inc = testStringIncludes();
    println!("  testStringIncludes() = {} (expected 0)", s_inc);

    let s_inc_nf = testStringIncludesNotFound();
    println!("  testStringIncludesNotFound() = {} (expected 0)", s_inc_nf);

    let s_sw = testStringStartsWith();
    println!("  testStringStartsWith() = {} (expected 0)", s_sw);

    let s_sw_f = testStringStartsWithFalse();
    println!("  testStringStartsWithFalse() = {} (expected 0)", s_sw_f);

    let s_ew = testStringEndsWith();
    println!("  testStringEndsWith() = {} (expected 0)", s_ew);

    let s_ew_f = testStringEndsWithFalse();
    println!("  testStringEndsWithFalse() = {} (expected 0)", s_ew_f);

    let s_trim = testStringTrim();
    println!("  testStringTrim() = {} (expected 0)", s_trim);

    // ── Math methods ──
    let m_abs = testMathAbs();
    println!("  testMathAbs() = {} (expected 0)", m_abs);

    let m_floor = testMathFloor();
    println!("  testMathFloor() = {} (expected 0)", m_floor);

    let m_ceil = testMathCeil();
    println!("  testMathCeil() = {} (expected 0)", m_ceil);

    let m_round = testMathRound();
    println!("  testMathRound() = {} (expected 0)", m_round);

    let m_max = testMathMax();
    println!("  testMathMax() = {} (expected 0)", m_max);

    let m_min = testMathMin();
    println!("  testMathMin() = {} (expected 0)", m_min);

    // ── Date methods ──
    let d_now = testDateNow();
    println!("  testDateNow() = {} (expected 0)", d_now);

    let d_from = testDateFromMillis();
    println!("  testDateFromMillis() = {} (expected 0)", d_from);

    let d_year = testDateGetFullYear();
    println!("  testDateGetFullYear() = {} (expected 0)", d_year);

    let d_month = testDateGetMonth();
    println!("  testDateGetMonth() = {} (expected 0)", d_month);

    let d_date = testDateGetDate();
    println!("  testDateGetDate() = {} (expected 0)", d_date);

    let d_day = testDateGetDay();
    println!("  testDateGetDay() = {} (expected 0)", d_day);

    let d_hours = testDateGetHours();
    println!("  testDateGetHours() = {} (expected 0)", d_hours);

    let d_now_s = testDateNowStatic();
    println!("  testDateNowStatic() = {} (expected 0)", d_now_s);

    let d_min = testDateGetMinutes();
    println!("  testDateGetMinutes() = {} (expected 0)", d_min);

    let d_sec = testDateGetSeconds();
    println!("  testDateGetSeconds() = {} (expected 0)", d_sec);

    let d_min_e = testDateMinutesEpoch();
    println!("  testDateMinutesEpoch() = {} (expected 0)", d_min_e);

    let d_sec_e = testDateSecondsEpoch();
    println!("  testDateSecondsEpoch() = {} (expected 0)", d_sec_e);

    let d_comp = testDateComposite();
    println!("  testDateComposite() = {} (expected 0)", d_comp);

    let d_day_v = testDateGetDayVerify();
    println!("  testDateGetDayVerify() = {} (expected 0)", d_day_v);

    // ── Number methods ──
    let pi = testParseInt();
    println!("  testParseInt() = {} (expected 0)", pi);

    let pi_hex = testParseIntHex();
    println!("  testParseIntHex() = {} (expected 0)", pi_hex);

    // ── Object methods ──
    let ok = testObjectKeys();
    println!("  testObjectKeys() = {} (expected 0)", ok);

    // ════════════════════════════════════════════════════════════
    // Phase 6 extra: Object spread merge
    println!("\n=== Spread Merge Tests ===");

    let ss = testSpreadSingle();
    println!("  testSpreadSingle() = {} (expected 0)", ss);

    let sm = testSpreadMulti();
    println!("  testSpreadMulti() = {} (expected 0)", sm);

    let st = testSpreadTriple();
    println!("  testSpreadTriple() = {} (expected 0)", st);

    let swi = testSpreadWithInline();
    println!("  testSpreadWithInline() = {} (expected 0)", swi);

    let so = testSpreadOverride();
    println!("  testSpreadOverride() = {} (expected 0)", so);

    // ════════════════════════════════════════════════════════════
    // Type inference verification: known limitations check
    println!("\n=== Type Inference Verification ===");

    // -- Division / Modulo (integer: @divTrunc / @rem) --
    let div = intDivTest();
    println!("  intDivTest(17/5 via assign) = {} (expected 3)", div);

    let mod_op = modOpTest();
    println!("  modOpTest(17%5 via assign) = {} (expected 2)", mod_op);

    let div_expr = testDivExpr();
    println!("  testDivExpr(17/5 expr) = {} (expected 3)", div_expr);

    let mod_expr = testModExpr();
    println!("  testModExpr(17%5 expr) = {} (expected 2)", mod_expr);

    // -- Bitwise & | ^ --
    let bw_and = testBitwiseAnd();
    println!("  testBitwiseAnd(12&10) = {} (expected 1)", bw_and);

    let bw_or = testBitwiseOr();
    println!("  testBitwiseOr(12|10) = {} (expected 1)", bw_or);

    let bw_xor = testBitwiseXor();
    println!("  testBitwiseXor(12^10) = {} (expected 1)", bw_xor);

    // -- Map.delete() / Set.delete() return value --
    let md = testMapDelete();
    println!("  testMapDelete() = {} (expected 1)", md);

    let sd = testSetDelete();
    println!("  testSetDelete() = {} (expected 1)", sd);

    // ========== P2: Destructuring Defaults ==========
    let dod = testDestructureObjDefault();
    println!("  testDestructureObjDefault() = {} (expected 1)", dod);

    let doe = testDestructureObjDefaultEmpty();
    println!("  testDestructureObjDefaultEmpty() = {} (expected 1)", doe);

    let dad = testDestructureArrDefault();
    println!("  testDestructureArrDefault() = {} (expected 1)", dad);

    let dae = testDestructureArrDefaultEmpty();
    println!("  testDestructureArrDefaultEmpty() = {} (expected 1)", dae);

    // ════════════════════════════════════════════════════════════
    // Class support tests
    println!("\n=== Class Support Tests ===");

    let ra = testRectArea();
    println!("  testRectArea() = {} (expected 12)", ra);

    let rp = testRectPerim();
    println!("  testRectPerim() = {} (expected 60)", rp);

    let uid = testUserId();
    println!("  testUserId() = {} (expected 42)", uid);

    let unl = testUserNameLength();
    println!("  testUserNameLength() = {} (expected 1)", unl);

    // ════════════════════════════════════════════════════════════
    // Static field + static block tests
    println!("\n=== Static Field & Block Tests ===");

    let sfr = testStaticFieldRead();
    println!("  testStaticFieldRead() = {} (expected 0)", sfr);

    let sfa = testStaticFieldAssign();
    println!("  testStaticFieldAssign() = {} (expected 0)", sfa);

    let sfm = testStaticFieldMultiply();
    println!("  testStaticFieldMultiply() = {} (expected 0)", sfm);

    let sfsg = testStaticFieldSetThenGet();
    println!("  testStaticFieldSetThenGet() = {} (expected 42)", sfsg);

    let sbi = testStaticBlockInit();
    println!("  testStaticBlockInit() = {} (expected 0)", sbi);

    let sbt = testStaticBlockThis();
    println!("  testStaticBlockThis() = {} (expected 0)", sbt);

    // ════════════════════════════════════════════════════════════
    // Dynamic array/string index tests
    println!("\n=== Dynamic Index Tests ===");

    let daa = testDynamicArrayAccess(2);
    println!("  testDynamicArrayAccess(2) = {} (expected 30)", daa);

    let das = testDynamicArrayAssign(1, 99);
    println!("  testDynamicArrayAssign(1,99) = {} (expected 99)", das);

    let dsum = testDynamicArraySum();
    println!("  testDynamicArraySum() = {} (expected 15)", dsum);

    let dsi = testDynamicStringIndex("Hello World", 0);
    println!(
        "  testDynamicStringIndex('Hello World',0) = {} (expected 1, checks byte 72='H')",
        dsi
    );

    let dsw = testDynamicArraySwap(0, 2);
    println!("  testDynamicArraySwap(0,2) = {} (expected 300)", dsw);

    // ════════════════════════════════════════════════════════════
    // Operator tests: delete
    println!("\n=== Operator Tests (delete) ===");

    let dmk = testDeleteMapKey();
    println!("  testDeleteMapKey() = {} (expected 1)", dmk);

    let dsk = testDeleteSetKey();
    println!("  testDeleteSetKey() = {} (expected 1)", dsk);

    // ── delete obj[key] (bracket syntax, was BUG-12) ──
    let dmb = testDeleteMapBracket();
    println!("  testDeleteMapBracket() = {} (expected 1)", dmb);

    let dmc = testDeleteMapComputedKey();
    println!("  testDeleteMapComputedKey() = {} (expected 1)", dmc);

    // ════════════════════════════════════════════════════════════
    // Operator tests: in (was BUG-01)
    println!("\n=== Operator Tests (in) ===");

    let iom = testInOperatorMap();
    println!("  testInOperatorMap() = {} (expected 1)", iom);

    let ios = testInOperatorSet();
    println!("  testInOperatorSet() = {} (expected 1)", ios);

    let ioo = testInOperatorObj();
    println!("  testInOperatorObj() = {} (expected 1)", ioo);

    // ════════════════════════════════════════════════════════════
    // Control flow tests: labeled statements
    println!("\n=== Control Flow Tests ===");

    let lb = testLabeledBreak();
    println!("  testLabeledBreak() = {} (expected 1)", lb);

    // ════════════════════════════════════════════════════════════
    // Array ES2023 tests (codegen limited, Rust unit tests cover fully)
    println!("\n=== Array ES2023 Tests ===");
    println!("  (Array ES2023 methods covered by 506 Rust unit tests)");

    // ════════════════════════════════════════════════════════════
    // Advanced built-in tests: Date UTC, Number statics, String methods
    println!("\n=== Advanced Built-in Tests ===");

    // Date UTC getters
    let ducy = testDateGetUTCFullYear();
    println!("  testDateGetUTCFullYear() = {} (expected 1)", ducy);

    let ducm = testDateGetUTCMonth();
    println!("  testDateGetUTCMonth() = {} (expected 1)", ducm);

    let ducd = testDateGetUTCDate();
    println!("  testDateGetUTCDate() = {} (expected 1)", ducd);

    // Number static methods
    let nif = testNumberIsFinite();
    println!("  testNumberIsFinite() = {} (expected 1)", nif);

    let nii = testNumberIsInteger();
    println!("  testNumberIsInteger() = {} (expected 1)", nii);

    // String slice/substring on parameter
    let ssl = testStringSliceParam("Hello World");
    println!("  testStringSliceParam('Hello World') = {:?}", ssl);

    let ssub = testStringSubstringParam("Mozilla");
    println!("  testStringSubstringParam('Mozilla') = {:?}", ssub);

    // ════════════════════════════════════════════════════════════
    // Advanced type tests: BigInt.asIntN/asUintN
    println!("\n=== Advanced Type Tests ===");

    let bain = testBigIntAsIntN();
    println!("  testBigIntAsIntN() = {} (expected 1)", bain);

    let baun = testBigIntAsUintN();
    println!("  testBigIntAsUintN() = {} (expected 1)", baun);

    // ════════════════════════════════════════════════════════════
    // Nullish coalescing tests (isolated file to avoid JsAny pollution)
    println!("\n=== Nullish Coalescing Tests ===");

    let nc = testNullishCoalescing();
    println!("  testNullishCoalescing() = {} (expected 1)", nc);

    let ncu = testNullishCoalescingUndefined();
    println!("  testNullishCoalescingUndefined() = {} (expected 1)", ncu);

    // ════════════════════════════════════════════════════════════
    // Advanced expression tests: unary +, bitwise compound assign
    println!("\n=== Advanced Expression Tests ===");

    let up = testUnaryPlus();
    println!("  testUnaryPlus() = {} (expected 1)", up);

    let bsla = testBitwiseShiftLeftAssign();
    println!("  testBitwiseShiftLeftAssign() = {} (expected 1)", bsla);

    let bsra = testBitwiseShiftRightAssign();
    println!("  testBitwiseShiftRightAssign() = {} (expected 1)", bsra);

    let baa = testBitwiseAndAssign();
    println!("  testBitwiseAndAssign() = {} (expected 1)", baa);

    let boa = testBitwiseOrAssign();
    println!("  testBitwiseOrAssign() = {} (expected 1)", boa);

    let bxa = testBitwiseXorAssign();
    println!("  testBitwiseXorAssign() = {} (expected 1)", bxa);

    // ════════════════════════════════════════════════════════════
    // Advanced statement tests: labeled for-of, nested try-catch
    println!("\n=== Advanced Statement Tests ===");

    let lfo = testLabeledForOf();
    println!("  testLabeledForOf() = {} (expected 1)", lfo);

    let ntc = testNestedTryCatch();
    println!("  testNestedTryCatch() = {:?} (expected Ok(1))", ntc);

    // ══════════════════════════════════════════════════════════════
    // JSDoc type annotation tests (Section 2.18)
    println!("\n=== JSDoc Type Annotation Tests ===");

    let ja = testJsdocArrayLength();
    println!("  testJsdocArrayLength() = {} (expected 1)", ja);

    let jo = testJsdocAnonObject();
    println!("  testJsdocAnonObject() = {} (expected 1)", jo);

    let jt = testJsdocTypedef();
    println!("  testJsdocTypedef() = {} (expected 1)", jt);

    // ══════════════════════════════════════════════════════════════
    // Logical assignment tests: &&= / ||= / ??= (was codegen bug)
    println!("\n=== Logical Assignment Tests ===");

    let aat = testAndAssignTruthy();
    println!("  testAndAssignTruthy() = {} (expected 10)", aat);

    let aaf = testAndAssignFalsy();
    println!("  testAndAssignFalsy() = {} (expected 0)", aaf);

    let oaf = testOrAssignFalsy();
    println!("  testOrAssignFalsy() = {} (expected 10)", oaf);

    let oat = testOrAssignTruthy();
    println!("  testOrAssignTruthy() = {} (expected 5)", oat);

    let nan = testNullishAssignNull();
    println!("  testNullishAssignNull() = {} (expected 1)", nan);

    // ── Sequence expression (comma operator) ──
    let seq = testSequenceExpr();
    println!("  testSequenceExpr() = {} (expected 3)", seq);

    // ══════════════════════════════════════════════════════════════
    // Private field and class expression tests
    println!("\n=== Private Field & Class Expression Tests ===");

    let pfi = testPrivateFieldInit();
    println!("  testPrivateFieldInit() = {} (expected 100)", pfi);

    let pfd = testPrivateFieldDefault();
    println!("  testPrivateFieldDefault() = {} (expected 0)", pfd);

    let tce = testClassExpression();
    println!("  testClassExpression() = {} (expected 10)", tce);

    let tcf = testClassExpressionFields();
    println!("  testClassExpressionFields() = {} (expected 10)", tcf);

    // ══════════════════════════════════════════════════════════════
    // for...of Map/Set/String collection tests
    println!("\n=== For-Of Collection Tests ===");

    let fms = testForOfMapSumValues();
    println!("  testForOfMapSumValues() = {} (expected 1)", fms);

    let fmk = testForOfMapKeyCheck();
    println!("  testForOfMapKeyCheck() = {} (expected 1)", fmk);

    let fss = testForOfSetSum();
    println!("  testForOfSetSum() = {} (expected 1)", fss);

    let fsc = testForOfStringCountOnly();
    println!("  testForOfStringCountOnly() = {} (expected 1)", fsc);

    let fsb = testForOfStringByteSum();
    println!("  testForOfStringByteSum() = {} (expected 1)", fsb);

    let mfe = testMapForEach();
    println!("  testMapForEach() = {} (expected 1)", mfe);

    let sfe = testSetForEach();
    println!("  testSetForEach() = {} (expected 1)", sfe);

    // ══════════════════════════════════════════════════════════════
    // for...in Map tests (iterates keys via .inner.iterator())
    println!("\n=== For-In Map Tests ===");

    let fimc = testForInMapCount();
    println!("  testForInMapCount() = {} (expected 1)", fimc);

    let fimk = testForInMapSingleKey();
    println!("  testForInMapSingleKey() = {} (expected 1)", fimk);

    // ══════════════════════════════════════════════════════════════
    // arguments object & rest parameter tests
    println!("\n=== Arguments & Rest Parameter Tests ===");

    let al = testArgumentsLength();
    println!("  testArgumentsLength() = {} (expected 0)", al);

    let aa = testArgumentsAccess(3, 7);
    println!("  testArgumentsAccess(3,7) = {} (expected 10)", aa);

    let ai = testArgumentsIterate(1, 2, 3);
    println!("  testArgumentsIterate(1,2,3) = {} (expected 6)", ai);

    let vs = testVariadicSum(1, 2, 3);
    println!("  testVariadicSum(1,2,3) = {} (expected 6)", vs);

    let vl = testVariadicLength(1, 2, 3, 4);
    println!("  testVariadicLength(1,2,3,4) = {} (expected 4)", vl);

    js2rust_deinit();
    println!("=== All tests done ===");
}
