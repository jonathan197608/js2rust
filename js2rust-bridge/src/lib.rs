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
// String conversion helpers
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
    let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
    c_str.to_str().ok()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(result.id, 5);
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
