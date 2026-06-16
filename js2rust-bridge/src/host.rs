//! Host functions — Rust implementations callable from translated JS/Zig code.
//!
//! These functions are exposed via the C ABI (`#[no_mangle] pub extern "C"`)
//! and referenced from Zig's `host.zig` as `extern "c"` declarations.
//!
//! To add a new host function:
//! 1. Define it here with `#[no_mangle] pub extern "C"`
//! 2. Register it in `core/src/main.rs` via `host_fns.register(...)` or `register_async(...)`
//! 3. The build pipeline generates the corresponding Zig `extern "c"` declaration

/// Simple addition — demo host function.
#[unsafe(no_mangle)]
pub extern "C" fn hostAdd(a: i64, b: i64) -> i64 {
    a + b
}

/// Simple multiplication — demo host function.
#[unsafe(no_mangle)]
pub extern "C" fn hostMultiply(a: i64, b: i64) -> i64 {
    a * b
}

// ── Async host function: fetchUser ──

/// Return type for async host function hostFetchUser.
/// C ABI struct: id + fixed-size name buffer.
#[repr(C)]
pub struct HostUserInfo {
    pub id: i64,
    pub name_buf: [u8; 128],
}

/// Async host function: fetches a user by name, returns id + greeting.
///
/// In Zig, this is called via an async wrapper that copies the result
/// into a clean struct. The fixed buffer avoids FFI string ownership issues.
///
/// # Safety
/// `name_ptr` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hostFetchUser(name_ptr: *const std::os::raw::c_char) -> HostUserInfo {
    // SAFETY: caller guarantees a valid null-terminated C string.
    let c_str = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
    let name = c_str.to_str().unwrap_or("unknown");
    let greeting = format!("Hello, {}!", name);

    let mut buf = [0u8; 128];
    let bytes = greeting.as_bytes();
    let len = bytes.len().min(127);
    buf[..len].copy_from_slice(&bytes[..len]);
    // buf[len] is already 0 (null terminator)

    // Simple demo: id = name length, name_buf = greeting
    HostUserInfo {
        id: name.len() as i64,
        name_buf: buf,
    }
}
