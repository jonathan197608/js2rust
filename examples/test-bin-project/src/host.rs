// src/host.rs
// Host functions — Rust implementations callable from translated JS/Zig code.
//
// These functions are exposed via the C ABI (`#[unsafe(no_mangle)] pub extern "C"`)
// and referenced from Zig's `host.zig` as `extern "c"` declarations.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
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

/// String concatenation — demo host function with string types.
#[unsafe(no_mangle)]
pub extern "C" fn host_concat(s1: *const c_char, s2: *const c_char) -> *mut c_char {
    let s1_str = unsafe {
        assert!(!s1.is_null());
        CStr::from_ptr(s1).to_string_lossy().into_owned()
    };

    let s2_str = unsafe {
        assert!(!s2.is_null());
        CStr::from_ptr(s2).to_string_lossy().into_owned()
    };

    let result = format!("{}{}", s1_str, s2_str);
    CString::new(result).unwrap().into_raw()
}

/// String length — demo host function with string type.
#[unsafe(no_mangle)]
pub extern "C" fn host_strlen(s: *const c_char) -> i64 {
    let s_str = unsafe {
        assert!(!s.is_null());
        CStr::from_ptr(s).to_string_lossy()
    };

    s_str.len() as i64
}

/// Free string memory allocated by Rust.
#[unsafe(no_mangle)]
pub extern "C" fn host_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

// ── Async host function (tokio-backed) ───────────────────────────

/// C ABI return struct for `fetch_user` (must match Zig's HostFetchUserResult).
#[repr(C)]
pub struct HostFetchUserResult {
    pub id: i64,
    pub name: [u8; 256],
}

/// Copy a Rust string into a fixed-size C buffer (null-terminated).
fn copy_to_buffer(buf: &mut [u8; 256], s: &str) {
    let bytes = s.as_bytes();
    let len = bytes.len().min(255);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0; // null terminator
}

/// Global tokio runtime — created once, reused for all async host calls.
/// Uses current-thread scheduler with all drivers enabled (time, I/O).
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
///
/// In a real application this would be an HTTP request, database query, or
/// filesystem read. Here we use `tokio::time::sleep` to emulate a 50ms
/// round-trip, then return a hardcoded record.
async fn fetch_user_from_db(name: &str) -> User {
    // Simulate network latency (50ms per query)
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Simulated database
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

/// C ABI wrapper: blocks on the async function using the global tokio runtime.
///
/// The Zig side calls this synchronously via `extern "c"`. Internally we
/// delegate to a real `async fn` running on a tokio runtime, then block
/// until the result is ready.
#[unsafe(no_mangle)]
pub extern "C" fn fetch_user(name: *const c_char) -> HostFetchUserResult {
    let name_str = unsafe {
        assert!(!name.is_null());
        CStr::from_ptr(name).to_string_lossy()
    };

    // Block on the async function — this is the standard bridge between
    // sync C ABI and async Rust. The tokio runtime drives the future to
    // completion (including the simulated sleep).
    let user = runtime().block_on(fetch_user_from_db(&name_str));

    let mut buf = [0u8; 256];
    copy_to_buffer(&mut buf, &user.name);
    HostFetchUserResult {
        id: user.id,
        name: buf,
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
