// sys: Rust FFI bindings for the generated Zig DLL.
//
// The build.rs script:
// 1. Runs the core JS->Zig pipeline (preprocess -> codegen -> testgen -> project gen)
// 2. Invokes `zig build` to produce js2rust.dll + js2rust.lib
// 3. Generates `ffi_bindings.rs` with extern "C" declarations from C ABI export metadata
//
// This file includes the generated bindings and provides safe Rust wrapper functions.

pub mod host;

// Include auto-generated FFI bindings from OUT_DIR
include!(concat!(env!("OUT_DIR"), "/ffi_bindings.rs"));

// ============================================================================
// Safe wrapper functions
// ============================================================================

/// Call the `applyCallback` Zig function.
/// Pure numeric I/O — no allocation concerns.
pub fn apply_callback(x: i64) -> i64 {
    // SAFETY: Zig C ABI, i64 params are trivially safe.
    unsafe { applyCallback(x) }
}

/// Call the `chineseAdd` Zig function.
/// Pure numeric I/O — no allocation concerns.
pub fn chinese_add(a: i64, b: i64) -> i64 {
    // SAFETY: Zig C ABI, i64 params are trivially safe.
    unsafe { chineseAdd(a, b) }
}

/// Call the `chineseSub` Zig function.
/// Pure numeric I/O — no allocation concerns.
pub fn chinese_sub(a: i64, b: i64) -> i64 {
    // SAFETY: Zig C ABI, i64 params are trivially safe.
    unsafe { chineseSub(a, b) }
}

// ============================================================================
// String conversion helpers (for future use when string exports are added)
// ============================================================================

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
    // SAFETY: caller guarantees ptr is a valid null-terminated C string.
    let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
    c_str.to_str().ok()
}

// ============================================================================
// Tests (run via `cargo test -p sys`)
//
// Two groups:
//   A. FFI wrapper tests — verify that the generated Zig DLL produces correct
//      results when called through the FFI wrapper functions.
//   B. Host function tests — verify that Rust host functions (callable from
//      JS/Zig via C ABI) produce correct results in pure Rust.
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── Group A: FFI wrapper tests ────────────────────────────────────

    #[test]
    fn test_chinese_add() {
        assert_eq!(chinese_add(3, 5), 8);
    }

    #[test]
    fn test_chinese_sub() {
        assert_eq!(chinese_sub(10, 3), 4); // helper(b) returns b*2
    }

    #[test]
    fn test_apply_callback() {
        assert_eq!(apply_callback(1), 2); // x + 1
    }

    // ── Group B: Rust host function tests ─────────────────────────────
    // These functions are exposed via C ABI for JS/Zig → Rust callbacks.

    #[test]
    fn test_host_add() {
        assert_eq!(host::hostAdd(10, 20), 30);
    }

    #[test]
    fn test_host_multiply() {
        assert_eq!(host::hostMultiply(6, 7), 42);
    }

    #[test]
    fn test_host_fetch_user() {
        let name = std::ffi::CString::new("Alice").unwrap();
        let result = unsafe { host::hostFetchUser(name.as_ptr()) };
        assert_eq!(result.id, 5); // "Alice" has 5 chars
        let name_bytes: Vec<u8> = result
            .name_buf
            .iter()
            .take_while(|&&b| b != 0)
            .copied()
            .collect();
        let greeting = String::from_utf8(name_bytes).unwrap();
        assert_eq!(greeting, "Hello, Alice!");
    }
}
