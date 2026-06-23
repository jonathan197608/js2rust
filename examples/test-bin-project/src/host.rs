// src/host.rs
// Host functions — Rust implementations callable from translated JS/Zig code.
//
// These functions are exposed via the C ABI (`#[unsafe(no_mangle)] pub extern "C"`)
// and referenced from Zig's `host.zig` as `extern "c"` declarations.
//
// Uses `js2rust_bridge::sdk` types for safe C ABI conversion:
// - `HostStr::from_raw(ptr, len)` — string params from Zig Arena → `&str`
// - `JsStr::new(&s)` — allocate return string in Zig Arena
// - `JsStrField::new(&s)` — string fields in async struct returns

use js2rust_bridge::sdk::{HostStr, JsStr, JsStrField};
use std::sync::OnceLock;
use std::time::Instant;
use tokio::runtime::Runtime;

// ── Synchronous host functions ───────────────────────────────────

/// Simple addition — demo host function.
#[unsafe(no_mangle)]
pub extern "C" fn host_add(a: i64, b: i64) -> i64 {
    a + b
}

/// Simple multiplication — demo host function.
#[unsafe(no_mangle)]
pub extern "C" fn host_multiply(a: i64, b: i64) -> i64 {
    a * b
}

/// String concatenation — SDK types handle all C ABI conversion.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_concat(
    s1_ptr: *const u8,
    s1_len: usize,
    s2_ptr: *const u8,
    s2_len: usize,
) -> JsStr {
    let s1 = HostStr::from_raw(s1_ptr, s1_len);
    let s2 = HostStr::from_raw(s2_ptr, s2_len);
    JsStr::new(&format!("{}{}", &s1, &s2))
}

/// String length — SDK type handles param conversion.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn host_strlen(s_ptr: *const u8, s_len: usize) -> i64 {
    let s = HostStr::from_raw(s_ptr, s_len);
    s.len() as i64
}

// ── Async host function (tokio-backed) ───────────────────────────

/// C ABI return struct for `fetch_user`.
///
/// `JsStrField` has the same `#[repr(C)]` layout as a ptr+len pair,
/// matching Zig's generated extern struct.  Fields are allocated in
/// Zig Arena via `JsStrField::new()`.
#[repr(C)]
pub struct HostFetchUserResult {
    pub id: i64,
    pub name: JsStrField,
}

/// Global tokio runtime — created once, reused for all async host calls.
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime")
    })
}

/// Simulated user record returned by the "database".
struct User {
    id: i64,
    name: String,
}

/// Async database lookup — simulates real async I/O with network latency.
async fn fetch_user_from_db(name: &str) -> User {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    match name {
        "alice" => User {
            id: 1,
            name: "Alice Smith".to_string(),
        },
        "bob" => User {
            id: 2,
            name: "Bob Jones".to_string(),
        },
        "charlie" => User {
            id: 3,
            name: "Charlie Brown".to_string(),
        },
        _ => User {
            id: 0,
            name: "Unknown User".to_string(),
        },
    }
}

/// C ABI wrapper for async fetch_user.
/// SDK types handle all ptr+len conversion — business logic is safe Rust.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fetch_user(
    name_ptr: *const u8,
    name_len: usize,
) -> HostFetchUserResult {
    let name = HostStr::from_raw(name_ptr, name_len);
    let user = runtime().block_on(fetch_user_from_db(&name));

    HostFetchUserResult {
        id: user.id,
        name: JsStrField::new(&user.name),
    }
}

/// Timing helper for benchmarking async calls from main.
#[allow(dead_code)]
pub fn timed<F: FnOnce() -> R, R>(label: &str, f: F) -> R {
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed();
    println!("  [timing] {} took {:?}", label, elapsed);
    result
}
