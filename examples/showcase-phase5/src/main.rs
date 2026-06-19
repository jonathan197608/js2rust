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

    // ── Array.splice (delete) ────────────────────
    println!();
    println!("--- Array.splice (delete) ---");
    let splice_del = testArraySpliceDelete_phase5();
    println!("  testArraySpliceDelete() = {} (expected 0)", splice_del);

    // ── Array.splice (delete + insert) ───────────
    println!();
    println!("--- Array.splice (delete + insert) ---");
    let splice_ins = testArraySpliceInsert_phase5();
    println!("  testArraySpliceInsert() = {} (expected 0)", splice_ins);

    js2rust_deinit();

    println!();
    println!("=== Phase 5 showcase completed ===");
}
