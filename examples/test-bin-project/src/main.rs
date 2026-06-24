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

    // ── Try-catch nesting tests ─────────────────────────────
    println!("\n── Try-catch nesting tests ──");
    let r1 = testNestedTryCatch_main().unwrap();
    println!("testNestedTryCatch_main() = {} (expected: 1012)", r1);
    assert_eq!(r1, 1012);

    let r2 = testNestedTryCatchWithThrow_main().unwrap();
    println!(
        "testNestedTryCatchWithThrow_main() = {} (expected: 1012)",
        r2
    );
    assert_eq!(r2, 1012);

    let r3 = testTryCatchWithResource_main().unwrap();
    println!("testTryCatchWithResource_main() = {} (expected: 43)", r3);
    assert_eq!(r3, 43);

    let r4 = testNestedTryCatchReThrow_main().unwrap();
    println!("testNestedTryCatchReThrow_main() = {} (expected: 1112)", r4);
    assert_eq!(r4, 1112);

    // ── Date tests ──────────────────────────────────────────
    println!("\n── Date tests ──");
    let d1 = testNewDate_main();
    println!("testNewDate_main() = {} (expected: > 0)", d1);
    assert!(
        d1 > 0,
        "testNewDate: expected positive timestamp, got {}",
        d1
    );

    let d2 = testNewDateWithMillis_main();
    println!("testNewDateWithMillis_main() = {} (expected: 1000)", d2);
    assert_eq!(d2, 1000);

    let d3 = testDateGetFullYear_main();
    println!("testDateGetFullYear_main() = {} (expected: 1970)", d3);
    assert_eq!(d3, 1970);

    let d4 = testDateGetDay_main();
    println!("testDateGetDay_main() = {} (expected: 4 = Thursday)", d4);
    assert_eq!(d4, 4);

    let d5 = testDateGetHours_main();
    println!("testDateGetHours_main() = {} (expected: 0)", d5);
    assert_eq!(d5, 0);

    let d6 = testDateGetMonth_main();
    println!("testDateGetMonth_main() = {} (expected: 0 = January)", d6);
    assert_eq!(d6, 0);

    let d7 = testDateGetDate_main();
    println!("testDateGetDate_main() = {} (expected: 1)", d7);
    assert_eq!(d7, 1);

    let d8 = testDateGetMinutes_main();
    println!("testDateGetMinutes_main() = {} (expected: 0)", d8);
    assert_eq!(d8, 0);

    let d9 = testDateGetSeconds_main();
    println!("testDateGetSeconds_main() = {} (expected: 0)", d9);
    assert_eq!(d9, 0);
}
