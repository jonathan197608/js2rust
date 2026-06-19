// js2rust-bridge: Rust FFI bindings for translated JS/Zig code.
//
// This crate re-exports the js2rust_bridge macro and provides
// host function implementations (Rust side of the FFI bridge).
//
// Users should call `js2rust_bridge!(group_name);` in their own code
// to generate FFI bindings for a specific transpilation group.

pub use js2rust_bridge_macro::js2rust_bridge;

pub mod host;

// === String conversion helpers ===

/// Convert a null-terminated C string pointer to a Rust &str.
///
/// # Safety
/// The pointer must be a valid, null-terminated C string allocated by Zig.
/// The returned &str borrows the memory; call the corresponding `free_*`
/// function after use to release the memory.
pub unsafe fn cstr_to_str<'a>(ptr: *const std::ffi::c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
    c_str.to_str().ok()
}
