// src/host.rs
// Host functions — Rust implementations callable from translated JS/Zig code.
//
// These functions are exposed via the C ABI (`#[unsafe(no_mangle)] pub extern "C"`)
// and referenced from Zig's `host.zig` as `extern "c"` declarations.
//
// Zero-copy design (v3.0):
// - String params: pass ptr+len directly from Zig Arena (no dupeZ)
// - String returns: allocate in Zig Arena via js_allocator_alloc() (no host_free)

use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

// ── C ABI types ──────────────────────────────────────────────────

/// Zero-copy string return type (matches Zig's `string.StrRet`).
/// ptr+len pair with sign-bit convention:
/// - len >= 0 → normal string of that length (memory in Zig Arena)
/// - len < 0  → panic/error, |len| bytes contain error name
#[repr(C)]
pub struct __JsStr {
    pub ptr: *const u8,
    pub len: isize,
}

impl __JsStr {
    /// Create a __JsStr from a Rust &str by allocating in Zig Arena.
    pub fn from_str(s: &str) -> Self {
        let len = s.len();
        let ptr = unsafe { js_allocator_alloc(len) };
        unsafe { std::ptr::copy_nonoverlapping(s.as_ptr(), ptr, len) };
        Self {
            ptr,
            len: len as isize,
        }
    }

    /// Create an empty __JsStr.
    pub fn empty() -> Self {
        Self {
            ptr: std::ptr::null(),
            len: 0,
        }
    }
}

extern "C" {
    /// Allocate memory in Zig's Arena for zero-copy string returns.
    fn js_allocator_alloc(size: usize) -> *mut u8;
}

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

/// String concatenation — zero-copy (params from Zig Arena, return in Zig Arena).
#[unsafe(no_mangle)]
pub extern "C" fn host_concat(
    s1_ptr: *const u8,
    s1_len: usize,
    s2_ptr: *const u8,
    s2_len: usize,
) -> __JsStr {
    let s1 = unsafe { std::str::from_utf8(std::slice::from_raw_parts(s1_ptr, s1_len)) };
    let s2 = unsafe { std::str::from_utf8(std::slice::from_raw_parts(s2_ptr, s2_len)) };

    let (s1, s2) = match (s1, s2) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return __JsStr::empty(),
    };

    let result = format!("{}{}", s1, s2);
    __JsStr::from_str(&result)
}

/// String length — zero-copy (param from Zig Arena).
#[unsafe(no_mangle)]
pub extern "C" fn host_strlen(s_ptr: *const u8, s_len: usize) -> i64 {
    let s = unsafe { std::str::from_utf8(std::slice::from_raw_parts(s_ptr, s_len)) };
    s.map(|s| s.len() as i64).unwrap_or(0)
}

// ── Async host function (tokio-backed) ───────────────────────────

/// C ABI return struct for `fetch_user` (must match Zig's HostFetchUserResult).
/// String fields use ptr+len pair — memory allocated in Zig Arena via js_allocator_alloc.
#[repr(C)]
pub struct HostFetchUserResult {
    pub id: i64,
    pub name_ptr: *const u8,
    pub name_len: usize,
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
    tokio::time::sleep(Duration::from_millis(50)).await;

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

/// C ABI wrapper for async fetch_user — zero-copy param from Zig Arena.
/// String return fields allocated in Zig Arena via js_allocator_alloc (zero-copy).
#[unsafe(no_mangle)]
pub extern "C" fn fetch_user(name_ptr: *const u8, name_len: usize) -> HostFetchUserResult {
    let name_str = unsafe {
        let slice = std::slice::from_raw_parts(name_ptr, name_len);
        std::str::from_utf8(slice).unwrap_or("unknown")
    };

    let user = runtime().block_on(fetch_user_from_db(name_str));

    // Allocate name in Zig Arena (zero-copy return)
    let name_bytes = user.name.as_bytes();
    let name_ptr_out = unsafe { js_allocator_alloc(name_bytes.len()) };
    unsafe { std::ptr::copy_nonoverlapping(name_bytes.as_ptr(), name_ptr_out, name_bytes.len()) };

    HostFetchUserResult {
        id: user.id,
        name_ptr: name_ptr_out,
        name_len: name_bytes.len(),
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
