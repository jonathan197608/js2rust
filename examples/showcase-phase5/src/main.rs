// src/main.rs
// Phase 5 showcase — pop/shift/reverse/sort/slice

use js2rust_bridge::js2rust_bridge;

js2rust_bridge! {
    "js_src/phase5.js",
}

fn main() {
    js2rust_init();

    println!("=== js2rust Phase 5 Showcase ===");
    println!();

    // ── Array.pop / shift ───────────────────────────
    println!("--- Array.pop / shift ---");
    let pop_val = testArrayPop_phase5();
    println!("  testArrayPop() = {} (expected 0)", pop_val);

    let shift_val = testArrayShift_phase5();
    println!("  testArrayShift() = {} (expected 0)", shift_val);

    // ── Array.reverse ─────────────────────────────
    println!();
    println!("--- Array.reverse ---");
    let rev_ok = testArrayReverse_phase5();
    println!("  testArrayReverse() = {} (expected 0)", rev_ok);

    // ── Array.sort ────────────────────────────────
    println!();
    println!("--- Array.sort ---");
    let sort_ok = testArraySort_phase5();
    println!("  testArraySort() = {} (expected 0)", sort_ok);

    // ── Array.slice ───────────────────────────────
    println!();
    println!("--- Array.slice ---");
    let slice_ok = testArraySlice_phase5();
    println!("  testArraySlice() = {} (expected 0)", slice_ok);

    // ── Array.map ────────────────────────────────
    println!();
    println!("--- Array.map ---");
    let map_ok = testArrayMap_phase5();
    println!("  testArrayMap() = {} (expected 0)", map_ok);

    // ── Array.filter ─────────────────────────────
    println!();
    println!("--- Array.filter ---");
    let filter_ok = testArrayFilter_phase5();
    println!("  testArrayFilter() = {} (expected 0)", filter_ok);

    // ── Array.reduce ─────────────────────────────
    println!();
    println!("--- Array.reduce ---");
    let reduce_ok = testArrayReduce_phase5();
    println!("  testArrayReduce() = {} (expected 0)", reduce_ok);

    // ── Array.some ───────────────────────────────
    println!();
    println!("--- Array.some ---");
    let some_ok = testArraySome_phase5();
    println!("  testArraySome() = {} (expected 0)", some_ok);

    // ── Array.every ──────────────────────────────
    println!();
    println!("--- Array.every ---");
    let every_ok = testArrayEvery_phase5();
    println!("  testArrayEvery() = {} (expected 0)", every_ok);

    js2rust_deinit();

    println!();
    println!("=== Phase 5 showcase completed ===");
}
