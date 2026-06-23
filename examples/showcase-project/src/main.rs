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

    // Force arena rotation (swap active/backup)
    js2rust_reset();
    println!("  js2rust_reset() called — arena rotated");

    // Verify correctness after reset
    let post_reduce = testArrayReduce_app();
    println!("  testArrayReduce (after reset) = {} (expected 0)", post_reduce);

    let post_add = testMemoryAdd_app(7, 8);
    println!("  testMemoryAdd(7,8) after reset = {} (expected 15)", post_add);

    let post_greeting = testLongGreeting_app("World");
    println!("  testLongGreeting('World') = {:?}", post_greeting);

    js2rust_deinit();
    println!("=== All tests done ===");
}
