// src/main.rs
// Test binary project for js2rust — JS-to-Zig transpiler

use js2rust_bridge::js2rust_bridge;
mod host; // Declare host module

// Generate FFI bindings: transpiles JS → Zig, generates Rust wrappers.
// No host functions for this minimal test.
js2rust_bridge! {
    "js_src/main.js",
}

fn main() {
    // Initialize Zig runtime (required for async export functions)
    js2rust_init();

    // ── Synchronous JS functions ──────────────────────────────
    let result = add_main(1, 2);
    println!("add_main(1, 2) = {}", result);

    // ── String return test (StrRet) ─────────────────────────
    let greeting = greet_main("World");
    println!("greet_main('World') = '{}'", greeting);

    // Cleanup
    js2rust_deinit();
}
