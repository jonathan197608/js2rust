//! Host function implementations for test-bin-project.
//!
//! Uses `#[host_fn]` attribute macro to eliminate all unsafe C ABI plumbing.
//! SDK types (`HostStr`, `JsStr`, `JsStrField`) handle pointer conversion.

use once_cell::sync::Lazy;
use tokio::runtime::Handle;

use js2rust_bridge::host_fn;
use js2rust_bridge::sdk::{HostStr, JsStr, JsStrField};

// ── Global tokio runtime singleton ─────────────────

static RUNTIME: Lazy<tokio::runtime::Runtime> =
    Lazy::new(|| tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"));

static HANDLE: Lazy<Handle> = Lazy::new(|| RUNTIME.handle().clone());

// ── Sync host functions ─────────────────────────────

/// Add two integers (no SDK types needed — plain C ABI).
#[host_fn]
fn host_add(a: i64, b: i64) -> i64 {
    a + b
}

/// Multiply two integers.
#[host_fn]
fn host_multiply(a: i64, b: i64) -> i64 {
    a * b
}

/// Concatenate two strings (SDK types: HostStr params, JsStr return).
#[host_fn]
fn host_concat(s1: HostStr, s2: HostStr) -> JsStr {
    JsStr::new(&format!("{s1}{s2}"))
}

/// Return string length.
#[host_fn]
fn host_strlen(s: HostStr) -> i64 {
    s.len() as i64
}

// ── Async host functions ────────────────────────────

/// Return struct for fetch_user async host function.
#[repr(C)]
pub struct FetchUserResult {
    pub id: i64,
    pub name: JsStrField,
}

/// Async host function: fetch user by name.
/// Uses global tokio runtime singleton to block on async operation (simulates DB query).
#[host_fn]
fn fetch_user(name: HostStr) -> FetchUserResult {
    // Use global tokio runtime singleton
    HANDLE.block_on(async {
        // Simulate async database query with 50ms delay
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        FetchUserResult {
            id: 42,
            name: JsStrField::new(&format!("User: {}", &name)),
        }
    })
}
