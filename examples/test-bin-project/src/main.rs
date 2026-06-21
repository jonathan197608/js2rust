// src/main.rs
// Test binary project for js2rust — JS-to-Zig transpiler

use js2rust_bridge::js2rust_bridge;
mod host;  // Declare host module

// Generate FFI bindings: transpiles JS → Zig, generates Rust wrappers.
// Host functions are declared inline — no build.rs needed for code generation.
js2rust_bridge! {
    "js_src/main.js",
    host_add(i64, i64) -> i64,
    host_multiply(i64, i64) -> i64,
    host_concat(str, str) -> str,
    host_strlen(str) -> i64,
    async fetch_user(str) -> { id: i64, name: str },
}

fn main() {
    // Initialize Zig runtime (required for async export functions)
    js2rust_init();

    // ── Synchronous JS functions ──────────────────────────────
    let result = greet_main("World");
    println!("greet_main('World') = {}", result);

    let sum = add_main(1, 2);
    println!("add_main(1, 2) = {}", sum);

    let product = multiply_main(3, 4);
    println!("multiply_main(3, 4) = {}", product);

    // ── Synchronous host functions (integer) ──────────────────
    let host_sum = useHostAdd_main(1, 2);
    println!("useHostAdd_main(1, 2) = {}", host_sum);

    let host_product = useHostMultiply_main(3, 4);
    println!("useHostMultiply_main(3, 4) = {}", host_product);

    // ── Synchronous host functions (string) ───────────────────
    let host_concat = useHostConcat_main("Hello, ", "World!");
    println!("useHostConcat_main('Hello, ', 'World!') = {}", host_concat);

    let host_strlen = useHostStrlen_main("Hello, World!");
    println!("useHostStrlen_main('Hello, World!') = {}", host_strlen);

    // ── TypedArray tests (Task #87) ── DISABLED: .length codegen fix pending
    // let ta_len = testNewInt32Array_main();
    // println!("testNewInt32Array_main() = {} (expect 3)", ta_len);
    //
    // let ta_u8_len = testNewUint8Array_main();
    // println!("testNewUint8Array_main() = {} (expect 5)", ta_u8_len);
    //
    // let ta_from_len = testInt32ArrayFrom_main();
    // println!("testInt32ArrayFrom_main() = {} (expect 3)", ta_from_len);

    // ── Async host function (tokio-backed) ─────────────────────
    println!("\n--- Async host function (tokio) ---");
    
    let user_name = host::timed("getUserInfo_main('alice')", || {
        getUserInfo_main("alice")
    });
    println!("  getUserInfo_main('alice') = {}", user_name);
    
    let user_name = host::timed("getUserInfo_main('bob')", || {
        getUserInfo_main("bob")
    });
    println!("  getUserInfo_main('bob') = {}", user_name);
    
    let two_users = host::timed("getTwoUserInfo_main('alice', 'charlie')", || {
        getTwoUserInfo_main("alice", "charlie")
    });
    println!("  getTwoUserInfo_main('alice', 'charlie') = {}", two_users);

    // Cleanup
    js2rust_deinit();
}
