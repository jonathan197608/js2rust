//! Safe SDK for writing host functions.
//!
//! These types encapsulate C ABI pointer conversion and Zig Arena memory allocation,
//! so host function implementations can be safe Rust with zero raw pointer manipulation.
//!
//! ## Quick reference
//!
//! | Type           | Direction  | C ABI signature               | Safe Rust type    |
//! |----------------|------------|-------------------------------|-------------------|
//! | [`HostStr`]    | Zig → Rust | `ptr: *const u8, len: usize`  | `&str`            |
//! | [`JsStr`]      | Rust → Zig | `-> JsStr` (ptr+len repr C)   | `String` / `&str` |
//! | [`JsStrField`] | Rust → Zig | struct field (ptr+len pair)   | `String` / `&str` |
//!
//! ## Example: sync string function
//!
//! ```rust,ignore
//! use js2rust_bridge::sdk::{HostStr, JsStr};
//!
//! #[unsafe(no_mangle)]
//! pub unsafe extern "C" fn host_concat(
//!     s1_ptr: *const u8, s1_len: usize,
//!     s2_ptr: *const u8, s2_len: usize,
//! ) -> JsStr {
//!     let s1 = HostStr::from_raw(s1_ptr, s1_len);
//!     let s2 = HostStr::from_raw(s2_ptr, s2_len);
//!     JsStr::new(&format!("{}{}", &s1, &s2))
//! }
//! ```
//!
//! ## Example: async struct return with string field
//!
//! ```rust,ignore
//! use js2rust_bridge::sdk::{HostStr, JsStrField};
//!
//! #[repr(C)]
//! pub struct MyResult {
//!     pub id: i64,
//!     pub name: JsStrField,
//! }
//!
//! #[unsafe(no_mangle)]
//! pub unsafe extern "C" fn fetch_user(
//!     name_ptr: *const u8, name_len: usize,
//! ) -> MyResult {
//!     let name = HostStr::from_raw(name_ptr, name_len);
//!     let user = runtime().block_on(lookup_user(&name));
//!     MyResult {
//!         id: user.id,
//!         name: JsStrField::new(&user.name),
//!     }
//! }
//! ```

use std::ops::Deref;

// ── String parameter from Zig Arena ────────────────────────────────

/// A string parameter received from Zig via C ABI (ptr+len pair).
///
/// Construct with [`HostStr::from_raw(ptr, len)`](Self::from_raw) inside your
/// `extern "C"` function.  Once constructed, use `&self` directly — it
/// implements [`Deref<Target = str>`](Deref).
///
/// The underlying memory is owned by Zig's Arena — do not store the
/// reference beyond the function call.
pub struct HostStr<'a>(&'a str);

impl<'a> HostStr<'a> {
    /// Create a `HostStr` from a C ABI ptr+len pair.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null for `len > 0` and point to valid UTF-8
    /// residing in Zig's Arena.  This is always true when called from a
    /// correctly generated C ABI wrapper.
    #[inline]
    pub unsafe fn from_raw(ptr: *const u8, len: usize) -> Self {
        if len == 0 || ptr.is_null() {
            return Self("");
        }
        // SAFETY: caller guarantees ptr points to valid UTF-8 in Zig Arena.
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        let s = std::str::from_utf8(slice).unwrap_or("");
        Self(s)
    }
}

impl<'a> Deref for HostStr<'a> {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        self.0
    }
}

impl<'a> AsRef<str> for HostStr<'a> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl<'a> std::fmt::Display for HostStr<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Debug for HostStr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// ── String return — allocated in Zig Arena ─────────────────────────

/// Allocate a string in Zig Arena via `js_allocator_dupe`.
///
/// Internal helper shared by `JsStr::new()` and `JsStrField::new()`.
/// Returns a null pointer on OOM (the Zig side returns null on allocation failure).
///
/// # Safety
///
/// The returned pointer must be used in accordance with Zig Arena lifetime rules.
unsafe fn dupe_to_arena(s: &str) -> *mut u8 {
    // Declared here to keep the extern block local (isolated from user code).
    // NOTE: return type is *mut u8 (not Option<*mut u8>) because Rust does NOT
    // guarantee null-pointer optimization for Option<*mut u8> — it would be
    // 16 bytes instead of 8, causing an ABI mismatch with Zig's ?[*]u8.
    unsafe extern "C" {
        fn js_allocator_dupe(src: *const u8, len: usize) -> *mut u8;
    }
    let ptr = unsafe { js_allocator_dupe(s.as_ptr(), s.len()) };
    assert!(
        !ptr.is_null(),
        "js_allocator_dupe returned null: Zig arena OOM"
    );
    ptr
}

/// Return type for sync host functions that return a string.
///
/// Memory is allocated in Zig's Arena via `js_allocator_dupe` (single call:
/// allocate + copy).  The Zig side receives ptr+len with zero-copy slicing.
///
/// Must be `#[repr(C)]` to match Zig's `extern struct { ptr, len }`.
#[repr(C)]
pub struct JsStr {
    /// Pointer to the string data in Zig Arena.
    pub ptr: *const u8,
    /// String length (non-negative for success).
    pub len: isize,
}

impl JsStr {
    /// Allocate a string in Zig Arena and return a `JsStr`.
    ///
    /// Uses `js_allocator_dupe` (single C ABI call: allocate + copy).
    /// The memory lives in Zig's Arena and is freed when the allocator is deinitialized.
    /// Panics on OOM (the Zig side returns null on allocation failure).
    #[inline]
    pub fn new(s: &str) -> Self {
        if s.is_empty() {
            return Self::empty();
        }
        let ptr = unsafe { dupe_to_arena(s) };
        Self {
            ptr,
            len: s.len() as isize,
        }
    }

    /// An empty string (null pointer, zero length).
    #[inline]
    pub fn empty() -> Self {
        Self {
            ptr: std::ptr::null(),
            len: 0,
        }
    }

    /// Returns true if this is an empty string.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// ── Async struct string field ──────────────────────────────────────

/// A string field inside an async host function's C ABI return struct.
///
/// Must be `#[repr(C)]` — its layout is `{ ptr: *const u8, len: usize }`,
/// matching Zig's generated extern struct.
///
/// ## Example
///
/// ```rust,ignore
/// #[repr(C)]
/// pub struct FetchUserResult {
///     pub id: i64,
///     pub name: JsStrField,
/// }
///
/// #[unsafe(no_mangle)]
/// pub unsafe extern "C" fn fetch_user(
///     name_ptr: *const u8, name_len: usize,
/// ) -> FetchUserResult {
///     let name = HostStr::from_raw(name_ptr, name_len);
///     // ... async lookup ...
///     FetchUserResult {
///         id: user.id,
///         name: JsStrField::new(&user.name),
///     }
/// }
/// ```
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct JsStrField {
    /// Pointer to the string data in Zig Arena.
    pub ptr: *const u8,
    /// String length in bytes.
    pub len: usize,
}

impl JsStrField {
    /// Allocate a string in Zig Arena and return a `JsStrField`.
    ///
    /// Uses `js_allocator_dupe` (single C ABI call: allocate + copy).
    /// Panics on OOM (the Zig side returns null on allocation failure).
    #[inline]
    pub fn new(s: &str) -> Self {
        if s.is_empty() {
            return Self::empty();
        }
        let ptr = unsafe { dupe_to_arena(s) };
        Self { ptr, len: s.len() }
    }

    /// An empty string field (null pointer, zero length).
    #[inline]
    pub fn empty() -> Self {
        Self {
            ptr: std::ptr::null(),
            len: 0,
        }
    }

    /// Returns true if this is an empty string.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// ── Test stubs for C ABI functions ─────────────────────────────
// When compiled as a static lib (e.g. for `cargo test`), the Zig runtime
// is not available, so we provide stubs for `js_allocator_dupe` and
// `js_allocator_alloc`. These stubs leak memory (like the old Box::leak),
// but tests don't run long enough for this to matter.
// Returns null on OOM to match the Zig-side C ABI contract.

#[cfg(any(test, feature = "stub-allocator"))]
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn js_allocator_dupe(src: *const u8, len: usize) -> *mut u8 {
    let layout = std::alloc::Layout::from_size_align(len, 1)
        .expect("Layout::from_size_align(len, 1) is infallible: align=1 is power-of-two and divides any size");
    let ptr = unsafe { std::alloc::alloc(layout) };
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    if len > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(src, ptr, len);
        }
    }
    ptr
}

#[cfg(any(test, feature = "stub-allocator"))]
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn js_allocator_alloc(size: usize) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    let layout = std::alloc::Layout::from_size_align(size, 1)
        .expect("Layout::from_size_align(size, 1) is infallible: align=1 is power-of-two and divides any size");
    let ptr = unsafe { std::alloc::alloc(layout) };
    if ptr.is_null() {
        std::ptr::null_mut()
    } else {
        ptr
    }
}
