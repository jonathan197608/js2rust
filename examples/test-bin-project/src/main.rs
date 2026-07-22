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
    let result = greet("World").unwrap();
    println!("greet('World') = {}", result);

    let sum = add(1.0, 2.0);
    println!("add(1, 2) = {}", sum);

    // ── Synchronous host functions (number) ───────────────────
    let host_sum = useHostAdd(1.0, 2.0);
    println!("useHostAdd(1, 2) = {}", host_sum);

    let host_product = useHostMultiply(3.0, 4.0);
    println!("useHostMultiply(3, 4) = {}", host_product);

    // ── Synchronous host functions (string) ───────────────────
    let host_concat = useHostConcat("Hello, ", "World!").unwrap();
    println!("useHostConcat('Hello, ', 'World!') = {}", host_concat);

    let host_strlen = useHostStrlen("Hello, World!");
    println!("useHostStrlen('Hello, World!') = {}", host_strlen);

    // ── Async host function (tokio-backed) ────────────────────
    // Test async host function fetch_user
    let user = getUserInfo("Alice");
    println!("getUserInfo('Alice') = id={}", user.id);

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

    // ── Try-catch nesting tests ─────────────────────────────
    println!("\n── Try-catch nesting tests ──");
    let r1 = testNestedTryCatch().unwrap();
    println!("testNestedTryCatch() = {} (expected: 1012)", r1);
    assert_eq!(r1, 1012.0);

    let r2 = testNestedTryCatchWithThrow().unwrap();
    println!("testNestedTryCatchWithThrow() = {} (expected: 1012)", r2);
    assert_eq!(r2, 1012.0);

    let r3 = testTryCatchWithResource().unwrap();
    println!("testTryCatchWithResource() = {} (expected: 43)", r3);
    assert_eq!(r3, 43.0);

    let r4 = testNestedTryCatchReThrow().unwrap();
    println!("testNestedTryCatchReThrow() = {} (expected: 1112)", r4);
    assert_eq!(r4, 1112.0);

    // ── Date tests ──────────────────────────────────────────
    println!("\n── Date tests ──");
    let d1 = testNewDate();
    println!("testNewDate() = {} (expected: > 0)", d1);
    assert!(
        d1 > 0.0,
        "testNewDate: expected positive timestamp, got {}",
        d1
    );

    let d2 = testNewDateWithMillis();
    println!("testNewDateWithMillis() = {} (expected: 1000)", d2);
    assert_eq!(d2, 1000.0);

    let d3 = testDateGetFullYear();
    println!("testDateGetFullYear() = {} (expected: 1970)", d3);
    assert_eq!(d3, 1970.0);

    let d4 = testDateGetDay();
    println!("testDateGetDay() = {} (expected: 4 = Thursday)", d4);
    assert_eq!(d4, 4.0);

    let d5 = testDateGetHours();
    println!("testDateGetHours() = {} (expected: 0)", d5);
    assert_eq!(d5, 0.0);

    let d6 = testDateGetMonth();
    println!("testDateGetMonth() = {} (expected: 0 = January)", d6);
    assert_eq!(d6, 0.0);

    let d7 = testDateGetDate();
    println!("testDateGetDate() = {} (expected: 1)", d7);
    assert_eq!(d7, 1.0);

    let d8 = testDateGetMinutes();
    println!("testDateGetMinutes() = {} (expected: 0)", d8);
    assert_eq!(d8, 0.0);

    let d9 = testDateGetSeconds();
    println!("testDateGetSeconds() = {} (expected: 0)", d9);
    assert_eq!(d9, 0.0);
}
