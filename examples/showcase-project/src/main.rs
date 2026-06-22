// src/main.rs — minimal showcase for testing mut_vars fix
use js2rust_bridge::js2rust_bridge;

// Transpile JS -> Zig and generate FFI bindings.
// app.js + phase5.js + test_throw.js
js2rust_bridge! {
    "js_src/app.js",
    "js_src/phase5.js",
    "js_src/test_throw.js",
}

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

    js2rust_deinit();
    println!("=== All tests done ===");
}
