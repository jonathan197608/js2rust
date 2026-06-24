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
    // Test async host function fetch_user
    let user = getUserInfo_main("Alice");
    println!("getUserInfo_main('Alice') = id={}", user.id);

    // Debug: print name field (JsStrField)
    println!(
        "  name.ptr = {:?}, name.len = {}",
        user.name.ptr, user.name.len
    );
    // Convert JsStrField to &str for printing
    let name_str = if user.name.len > 0 && !user.name.ptr.is_null() {
        let slice = unsafe { std::slice::from_raw_parts(user.name.ptr, user.name.len) };
        std::str::from_utf8(slice).unwrap_or("(invalid utf-8)")
    } else {
        "(empty)"
    };
    println!("  name = {}", name_str);

    // Cleanup
    js2rust_deinit();
}
