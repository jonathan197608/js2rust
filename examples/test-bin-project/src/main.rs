// src/main.rs
// Test binary project for js2rust — JS-to-Zig transpiler

use js2rust_bridge::js2rust_bridge;
mod host; // Declare host module

// Generate FFI bindings: transpiles JS → Zig, generates Rust wrappers.
// All configuration is read from js2rust.toml.
js2rust_bridge!();

fn main() {
    // Initialize Zig runtime (required for async export functions)
    js2rust_init();

    // ── Synchronous JS functions ──────────────────────────────
    let result = greet_main("World").unwrap();
    println!("greet_main('World') = {}", result);

    let sum = add_main(1, 2);
    println!("add_main(1, 2) = {}", sum);

    // ── Synchronous host functions (integer) ──────────────────
    let host_sum = useHostAdd_main(1, 2);
    println!("useHostAdd_main(1, 2) = {}", host_sum);

    let host_product = useHostMultiply_main(3, 4);
    println!("useHostMultiply_main(3, 4) = {}", host_product);

    // ── Synchronous host functions (string) ───────────────────
    let host_concat = useHostConcat_main("Hello, ", "World!").unwrap();
    println!("useHostConcat_main('Hello, ', 'World!') = {}", host_concat);

    let host_strlen = useHostStrlen_main("Hello, World!");
    println!("useHostStrlen_main('Hello, World!') = {}", host_strlen);

    // ── Async host function (tokio-backed) ────────────────────
    println!("\n--- Async host function (tokio) ---");

    // Single user lookup — should take ~50ms (simulated network latency)
    let user_name = host::timed("getUserInfo_main('alice')", || {
        getUserInfo_main("alice").unwrap()
    });
    println!("  getUserInfo_main('alice') = {}", user_name);

    // Another lookup
    let user_name = host::timed("getUserInfo_main('bob')", || {
        getUserInfo_main("bob").unwrap()
    });
    println!("  getUserInfo_main('bob') = {}", user_name);

    // Two sequential lookups — should take ~100ms total (2 x 50ms)
    let two_users = host::timed("getTwoUserInfo_main('alice', 'charlie')", || {
        getTwoUserInfo_main("alice", "charlie").unwrap()
    });
    println!("  getTwoUserInfo_main('alice', 'charlie') = {}", two_users);

    // Cleanup
    js2rust_deinit();
}
