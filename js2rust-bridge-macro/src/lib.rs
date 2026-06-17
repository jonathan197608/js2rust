//! Proc-macro for generating Rust FFI bindings from js2rust C ABI export metadata.
//!
//! Usage:
//! ```rust,ignore
//! // In js2rust-bridge/src/lib.rs:
//! use js2rust_bridge_macro::js2rust_bridge;
//! js2rust_bridge!("out/main/cabi_exports.json");
//! ```
//!
//! Each macro invocation corresponds to one JS file group.
//! The macro reads the JSON file at compile time and generates:
//! 1. `unsafe extern "C"` block with raw FFI declarations (names match Zig exports exactly)
//! 2. Safe Rust wrapper functions with `_<group>` suffix (e.g. `greet_main`, `mathRound_builtins`)
//!
//! The group name is derived from the JSON file path:
//! `out/main/cabi_exports.json` → suffix `_main`
//! `out/builtins/cabi_exports.json` → suffix `_builtins`

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;

/// Find the workspace root by looking for a Cargo.toml with [workspace].
fn find_workspace_root(start: &str) -> String {
    let mut current = std::path::PathBuf::from(start);
    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return current.to_string_lossy().to_string();
                }
            }
        }
        if !current.pop() {
            return std::path::PathBuf::from(start)
                .parent()
                .unwrap()
                .to_string_lossy()
                .to_string();
        }
    }
}

/// Extract group name from the JSON file path.
/// e.g. `out/main/cabi_exports.json` → `main`
///        `out/my-group/cabi_exports.json` → `my_group`
fn extract_group_name(path: &std::path::Path) -> String {
    let raw = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    sanitize_ident(&raw)
}

/// Sanitize a string into a valid Rust identifier fragment.
/// Replaces any char that is not ASCII alphanumeric or `_` with `_`.
/// Prepends `_` if the first char is a digit.
fn sanitize_ident(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    // Rust identifiers cannot start with a digit
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out = format!("_{}", out);
    }
    if out.is_empty() {
        out.push_str("unknown");
    }
    out
}

/// C ABI export metadata (mirrors the JSON schema written by js2rustc).
#[derive(Debug, Deserialize)]
struct CabiExport {
    name: String,
    params: Vec<CabiParam>,
    ret_type: String,
    has_free_func: bool,
}

#[derive(Debug, Deserialize)]
struct CabiParam {
    #[allow(dead_code)]
    name: String,
    zig_type: String,
}

/// Function-like proc-macro: `js2rust_bridge!("path/to/cabi_exports.json");`
///
/// Generates FFI bindings + safe wrappers for one group.
#[proc_macro]
pub fn js2rust_bridge(input: TokenStream) -> TokenStream {
    // Parse the input as a single string literal
    let lit_str = match syn::parse::<syn::LitStr>(input) {
        Ok(s) => s,
        Err(e) => return e.to_compile_error().into(),
    };

    let json_path = lit_str.value();

    // Resolve path: the path in the macro invocation is relative to the workspace root.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = find_workspace_root(&manifest_dir);
    let resolved_path = std::path::Path::new(&workspace_root).join(&json_path);

    // Extract group name from path (used as suffix for generated functions)
    let group_name = extract_group_name(&resolved_path);

    // Read and parse JSON
    let json_content = match std::fs::read_to_string(&resolved_path) {
        Ok(s) => s,
        Err(e) => {
            return syn::Error::new(
                lit_str.span(),
                format!(
                    "js2rust_bridge: cannot read '{}': {}\nResolved path: {}",
                    json_path,
                    e,
                    resolved_path.display()
                ),
            )
            .to_compile_error()
            .into();
        }
    };

    let exports: Vec<CabiExport> = match serde_json::from_str(&json_content) {
        Ok(v) => v,
        Err(e) => {
            return syn::Error::new(
                lit_str.span(),
                format!("js2rust_bridge: failed to parse '{}': {}", json_path, e),
            )
            .to_compile_error()
            .into();
        }
    };

    // Generate code with the group name as suffix for functions
    // and as the module name (instead of a hash).
    let generated = generate_bindings(&exports, &group_name);

    match generated.parse::<TokenStream>() {
        Ok(ts) => ts,
        Err(e) => syn::Error::new(lit_str.span(), format!("internal error: {}", e))
            .to_compile_error()
            .into(),
    }
}

/// Generate Rust FFI bindings + safe wrappers from C ABI export metadata.
///
/// `group_suffix` is appended to all safe wrapper function names to avoid
/// inter-group name collisions (e.g. `greet` → `greet_main`).
/// It is also used in the raw/safe module names so they are human-readable.
fn generate_bindings(exports: &[CabiExport], group_suffix: &str) -> String {
    let mut extern_fns = Vec::new();
    let mut safe_wrappers = Vec::new();

    let raw_mod = format_ident!("__js2rust_ffi_raw_{group_suffix}");
    let safe_mod = format_ident!("__js2rust_ffi_safe_{group_suffix}");

    for exp in exports {
        let fn_name = format_ident!("{}", exp.name);
        let free_fn_name = format_ident!("free_{}", exp.name);

        // Build parameter list for extern declaration
        let mut extern_params = Vec::new();
        let mut safe_params = Vec::new();
        let mut call_args = Vec::new();

        for (idx, param) in exp.params.iter().enumerate() {
            let param_ident = format_ident!("arg{}", idx);
            let param_ty = zig_type_to_rust_ffi_type(&param.zig_type);
            extern_params.push(quote! { #param_ident: #param_ty });
            safe_params.push(quote! { #param_ident: #param_ty });
            call_args.push(quote! { #param_ident });
        }

        let ret_ty = zig_ret_type_to_rust_ffi(&exp.ret_type);

        // Generate `unsafe extern "C"` declaration
        extern_fns.push(quote! {
            pub fn #fn_name( #(#extern_params),* ) -> #ret_ty;
        });

        if exp.has_free_func {
            extern_fns.push(quote! {
                pub fn #free_fn_name(ptr: *mut std::ffi::c_void);
            });
        }

        // Generate safe wrapper (with group suffix to avoid name collisions)
        let safe_wrapper = generate_safe_wrapper(exp, &fn_name, &free_fn_name, &raw_mod, group_suffix);
        safe_wrappers.push(safe_wrapper);
    }

    // Output: separate mod for raw FFI, then safe wrappers at top level
    let output = quote! {
        #[allow(non_snake_case)]
        #[allow(dead_code)]
        mod #raw_mod {
            unsafe extern "C" {
                #(#extern_fns)*
            }
        }

        #[allow(non_snake_case)]
        #[allow(dead_code)]
        mod #safe_mod {
            use super::#raw_mod;

            #(#safe_wrappers)*
        }

        // Re-export safe wrappers at the invocation site
        pub use #safe_mod::*;
    };

    output.to_string()
}

/// Generate a safe Rust wrapper function for a C ABI export.
///
/// The wrapper function name gets `_group_suffix` appended so that
/// multiple groups can be imported without name collisions.
fn generate_safe_wrapper(
    exp: &CabiExport,
    fn_name: &syn::Ident,
    free_fn_name: &syn::Ident,
    raw_mod: &syn::Ident,
    group_suffix: &str,
) -> proc_macro2::TokenStream {
    // Safe wrapper name: `greet` → `greet_main`
    let wrapper_name = format_ident!("{}_{}", exp.name, group_suffix);
    let mut safe_params = Vec::new();
    let mut ffi_args = Vec::new();

    // Build safe parameter list (convert &str → *const c_char if needed)
    for (idx, param) in exp.params.iter().enumerate() {
        let param_ident = format_ident!("arg{}", idx);
        let safe_ty = zig_type_to_rust_safe_type(&param.zig_type);
        safe_params.push(quote! { #param_ident: #safe_ty });
        ffi_args.push(convert_safe_to_ffi(&param.zig_type, &param_ident));
    }

    let (ret_ty, call_expr) = if exp.ret_type == "[]const u8" {
        // String return: call FFI, convert to String, free
        (
            quote! { String },
            quote! {
                {
                    let ptr = unsafe { super::#raw_mod::#fn_name(#(#ffi_args),*) };
                    if ptr.is_null() {
                        String::new()
                    } else {
                        let s = unsafe {
                            std::ffi::CStr::from_ptr(ptr)
                                .to_string_lossy()
                                .into_owned()
                        };
                        unsafe { super::#raw_mod::#free_fn_name(ptr as *mut std::ffi::c_void) };
                        s
                    }
                }
            },
        )
    } else {
        let rust_ret = zig_ret_type_to_rust_safe(&exp.ret_type);
        (
            rust_ret.clone(),
            quote! {
                unsafe { super::#raw_mod::#fn_name(#(#ffi_args),*) }
            },
        )
    };

    quote! {
        #[allow(non_snake_case)]
        pub fn #wrapper_name( #(#safe_params),* ) -> #ret_ty {
            #call_expr
        }
    }
}

/// Convert a Zig type string to Rust FFI type (for `unsafe extern "C"`).
fn zig_type_to_rust_ffi_type(zig_type: &str) -> proc_macro2::TokenStream {
    match zig_type {
        "i64" | "i32" | "usize" => quote! { i64 },
        "f64" | "f32" => quote! { f64 },
        "bool" => quote! { bool },
        "[]const u8" => quote! { *const std::ffi::c_char },
        "void" => quote! { () },
        _ => {
            // Default: treat as opaque pointer
            quote! { *const std::ffi::c_void }
        }
    }
}

/// Convert a Zig return type to Rust FFI return type.
fn zig_ret_type_to_rust_ffi(zig_type: &str) -> proc_macro2::TokenStream {
    match zig_type {
        "[]const u8" => quote! { *mut std::ffi::c_char },
        _ => zig_type_to_rust_ffi_type(zig_type),
    }
}

/// Convert a Zig type to safe Rust type (for wrapper function signatures).
fn zig_type_to_rust_safe_type(zig_type: &str) -> proc_macro2::TokenStream {
    match zig_type {
        "i64" | "i32" | "usize" => quote! { i64 },
        "f64" | "f32" => quote! { f64 },
        "bool" => quote! { bool },
        "[]const u8" => quote! { &str },
        "void" => quote! { () },
        _ => quote! { i64 },
    }
}

/// Convert safe Rust type to FFI type (for function call arguments).
fn convert_safe_to_ffi(zig_type: &str, ident: &syn::Ident) -> proc_macro2::TokenStream {
    match zig_type {
        "[]const u8" => {
            quote! {
                std::ffi::CString::new(#ident)
                    .expect("null byte in string")
                    .as_ptr()
            }
        }
        _ => quote! { #ident },
    }
}

/// Convert a Zig return type to safe Rust return type.
fn zig_ret_type_to_rust_safe(zig_type: &str) -> proc_macro2::TokenStream {
    match zig_type {
        "[]const u8" => quote! { String },
        "i64" | "i32" | "usize" => quote! { i64 },
        "f64" | "f32" => quote! { f64 },
        "bool" => quote! { bool },
        "void" => quote! { () },
        _ => quote! { i64 },
    }
}
