//! Proc-macro for generating Rust FFI bindings from js2rust C ABI export metadata.
//!
//! ## Usage
//!
//! ```rust,ignore
//! js2rust_bridge! {
//!     "js_src/main.js",    // core JS file path (relative to CARGO_MANIFEST_DIR)
//!     // Sync host functions (optional, comma-separated):
//!     host_add(i64, i64) -> i64,
//!     host_concat(str, str) -> str,
//!     // Async host functions (called with `await` from JS):
//!     async fetch_user(str) -> { id: i64, name: str },
//! }
//! ```
//!
//! The macro transpiles JS to Zig inline, writes output to
//! `.js2zig-cache/{group}/`, and generates Rust FFI bindings.
//! The group name is derived from the file name (sanitized for Zig identifiers).
//! A minimal `build.rs` is only needed to link the compiled static library.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;
use syn::{
    braced, parenthesized, parse::{Parse, ParseStream}, Ident, LitStr, Token
};

// ── C ABI export metadata (mirrors the JSON schema) ───────────────

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

// ── Macro input parsing ───────────────────────────────────────────

/// Parsed host function declaration: `[async] name(type1, type2) -> ret_type`
struct HostFnDecl {
    name: String,
    params: Vec<String>,
    return_type: String,
    is_async: bool,
    /// For async functions with struct return: Vec<(field_name, field_type)>
    async_return_fields: Vec<(String, String)>,
}

/// Full macro input.
struct MacroInput {
    /// Core JS file path (e.g. "js_src/main.js").
    js_file: String,
    /// Group name derived from the file stem (sanitized).
    group: String,
    host_fns: Vec<HostFnDecl>,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Core JS file path (string literal)
        let js_file_lit: LitStr = input.parse()?;
        let js_file = js_file_lit.value();

        // Derive group name from file stem, sanitized for Zig identifiers.
        let stem = std::path::Path::new(&js_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main");
        let group = js2zig_core::analyzer::sanitize_module_name(stem);

        // Optional host function declarations
        let mut host_fns = Vec::new();
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }

            // Parse first — could be `async` keyword or function name
            let is_async = input.peek(Token![async]);
            if is_async {
                input.parse::<Token![async]>()?;
            }
            let name: Ident = input.parse()?;
            let name_str = name.to_string();

            // parameter types in parentheses
            let paren_content;
            parenthesized!(paren_content in input);
            let mut params = Vec::new();
            while !paren_content.is_empty() {
                let ty: Ident = paren_content.parse()?;
                params.push(ty.to_string());
                if paren_content.peek(Token![,]) {
                    paren_content.parse::<Token![,]>()?;
                }
            }

            // return type after `->`
            input.parse::<Token![->]>()?;

            // Check for struct return type `{ field: type, ... }` or simple Ident
            let (return_type, async_fields) = if input.peek(syn::token::Brace) {
                let struct_content;
                braced!(struct_content in input);
                let mut fields = Vec::new();
                while !struct_content.is_empty() {
                    let field_name: Ident = struct_content.parse()?;
                    struct_content.parse::<Token![:]>()?;
                    let field_type: Ident = struct_content.parse()?;
                    fields.push((field_name.to_string(), field_type.to_string()));
                    if struct_content.peek(Token![,]) {
                        struct_content.parse::<Token![,]>()?;
                    }
                }
                // For struct returns, return_type is "void" (actual type is in async_fields)
                ("void".to_string(), fields)
            } else {
                let ret: Ident = input.parse()?;
                (ret.to_string(), Vec::new())
            };

            host_fns.push(HostFnDecl {
                name: name_str,
                params,
                return_type,
                is_async,
                async_return_fields: async_fields,
            });
        }

        Ok(MacroInput {
            js_file,
            group,
            host_fns,
        })
    }
}

// ── Type name conversion ──────────────────────────────────────────

/// Convert macro-level type name to `js2zig_core::HostType`.
fn type_name_to_host_type(name: &str) -> Result<js2zig_core::HostType, String> {
    match name {
        "i64" => Ok(js2zig_core::HostType::I64),
        "i32" => Ok(js2zig_core::HostType::I32),
        "f64" => Ok(js2zig_core::HostType::F64),
        "bool" => Ok(js2zig_core::HostType::Bool),
        "str" => Ok(js2zig_core::HostType::Str),
        "void" => Ok(js2zig_core::HostType::Void),
        other => Err(format!("js2rust_bridge: unknown host type '{}'. \
            Valid types: i64, i32, f64, bool, str, void", other)),
    }
}

// ── Main proc-macro entry point ───────────────────────────────────

/// Function-like proc-macro: `js2rust_bridge!("js_src/main.js", host_fns...)`.
///
/// Transpiles JS to Zig, generates Rust FFI bindings, and optionally
/// runs `zig build` to compile the static library.
#[proc_macro]
pub fn js2rust_bridge(input: TokenStream) -> TokenStream {
    let input_tokens: proc_macro2::TokenStream = input.into();

    match syn::parse2::<MacroInput>(input_tokens) {
        Ok(parsed) => generate(&parsed),
        Err(e) => e.to_compile_error().into(),
    }
}

// ── Transpile + generate FFI ──────────────────────────────────────

fn generate(input: &MacroInput) -> TokenStream {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());

    // Resolve core JS file path
    let js_file_path = std::path::Path::new(&manifest_dir).join(&input.js_file);

    // Resolve cache directory for Zig output
    let cache_dir = std::path::Path::new(&manifest_dir)
        .join(".js2zig-cache");

    // Convert host function declarations to js2zig_core::HostFunction
    let mut host_functions = Vec::new();
    for hf in &input.host_fns {
        let params: Result<Vec<_>, _> = hf.params.iter()
            .map(|t| type_name_to_host_type(t))
            .collect();
        let params = match params {
            Ok(p) => p,
            Err(e) => return syn::Error::new(proc_macro2::Span::call_site(), e)
                .to_compile_error().into(),
        };

        let return_type = match type_name_to_host_type(&hf.return_type) {
            Ok(js2zig_core::HostType::Void) => None,
            Ok(t) => Some(t),
            Err(e) => return syn::Error::new(proc_macro2::Span::call_site(), e)
                .to_compile_error().into(),
        };

        // Convert async return fields
        let async_return_fields: Result<Vec<_>, _> = hf.async_return_fields.iter()
            .map(|(name, ty)| {
                type_name_to_host_type(ty).map(|t| (name.clone(), t))
            })
            .collect();
        let async_return_fields = match async_return_fields {
            Ok(v) => v,
            Err(e) => return syn::Error::new(proc_macro2::Span::call_site(), e)
                .to_compile_error().into(),
        };

        host_functions.push(js2zig_core::HostFunction {
            name: hf.name.clone(),
            params,
            return_type,
            is_async: hf.is_async,
            async_return_fields,
        });
    }

    // Build ProjectConfig
    let config = js2zig_core::ProjectConfig {
        name: input.group.clone(),
        js_file: js_file_path.clone(),
        out_dir: cache_dir.clone(),
        host_config: if host_functions.is_empty() {
            None
        } else {
            Some(js2zig_core::HostConfig {
                functions: host_functions,
            })
        },
        force_rebuild: false,
        run_zig_build: false,
    };

    // Transpile!
    let project_result = match js2zig_core::transpile_project(&config) {
        Ok(result) => result,
        Err(e) => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("js2rust_bridge: transpilation failed: {}", e),
            )
            .to_compile_error()
            .into();
        }
    };

    // Find the group result (there is exactly one group from analyze_single_group)
    let group_result = project_result.groups.first();

    let group_result = match group_result {
        Some(g) => g,
        None => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "js2rust_bridge: no groups found in transpilation result",
            )
            .to_compile_error()
            .into();
        }
    };

    // Parse cabi_exports_json
    let exports: Vec<CabiExport> = match serde_json::from_str(&group_result.cabi_exports_json) {
        Ok(v) => v,
        Err(e) => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("js2rust_bridge: failed to parse cabi_exports: {}", e),
            )
            .to_compile_error()
            .into();
        }
    };

    // Optionally run zig build (side effect — generates static library for linking)
    let zig_project_dir = cache_dir.join(&input.group);
    if zig_project_dir.join("build.zig").exists() {
        let _ = std::process::Command::new("zig")
            .arg("build")
            .current_dir(&zig_project_dir)
            .status();
    }

    // Generate Rust FFI bindings from cabi exports
    let generated = generate_bindings(&exports, &input.group);

    match generated.parse::<TokenStream>() {
        Ok(ts) => ts,
        Err(e) => syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("internal error: {}", e),
        )
        .to_compile_error()
        .into(),
    }
}

// ── FFI bindings generation ───────────────────────────────────────

fn generate_bindings(exports: &[CabiExport], group_suffix: &str) -> String {
    let mut extern_fns = Vec::new();
    let mut safe_wrappers = Vec::new();
    let mut needs_free_string = false;

    let raw_mod = format_ident!("__js2rust_ffi_raw_{group_suffix}");
    let safe_mod = format_ident!("__js2rust_ffi_safe_{group_suffix}");

    for exp in exports {
        let fn_name = format_ident!("{}", exp.name);
        let free_fn_name = format_ident!("free_{}", exp.name);

        let mut extern_params = Vec::new();
        for param in &exp.params {
            let param_ident = format_ident!("{}", param.name);
            let param_ty = zig_type_to_rust_ffi_type(&param.zig_type);
            extern_params.push(quote! { #param_ident: #param_ty });
        }

        let ret_ty = zig_ret_type_to_rust_ffi(&exp.ret_type);

        extern_fns.push(quote! {
            pub fn #fn_name( #(#extern_params),* ) -> #ret_ty;
        });

        if exp.has_free_func || exp.ret_type == "[*c]u8" {
            needs_free_string = true;
        }

        let safe_wrapper = generate_safe_wrapper(exp, &fn_name, &free_fn_name, &raw_mod, group_suffix);
        safe_wrappers.push(safe_wrapper);
    }

    // Always provide js2rust_init/deinit safe wrappers (C ABI exports from lib.zig)
    let runtime_init = quote! {
        /// Initialize the Zig runtime (allocator + Io for async functions).
        /// Call this before any async export function.
        pub fn js2rust_init() {
            extern "C" {
                #[link_name = "js2rust_init"]
                fn _js2rust_init();
            }
            unsafe { _js2rust_init() };
        }
        /// Release Zig runtime resources.
        pub fn js2rust_deinit() {
            extern "C" {
                #[link_name = "js2rust_deinit"]
                fn _js2rust_deinit();
            }
            unsafe { _js2rust_deinit() };
        }
    };
    safe_wrappers.push(runtime_init);

    // Generate free_string() extern declaration if needed
    if needs_free_string {
        extern_fns.push(quote! {
            pub fn free_string(ptr: *mut std::ffi::c_char, len: usize);
        });
    }

    let output = quote! {
        #[allow(non_snake_case)]
        #[allow(dead_code)]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        mod #raw_mod {
            unsafe extern "C" {
                #(#extern_fns)*
            }
        }

        #[allow(non_snake_case)]
        #[allow(dead_code)]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        mod #safe_mod {
            use super::#raw_mod;

            #(#safe_wrappers)*
        }

        pub use #safe_mod::*;
    };

    output.to_string()
}

fn generate_safe_wrapper(
    exp: &CabiExport,
    fn_name: &syn::Ident,
    free_fn_name: &syn::Ident,
    raw_mod: &syn::Ident,
    group_suffix: &str,
) -> proc_macro2::TokenStream {
    let wrapper_name = format_ident!("{}_{}", exp.name, group_suffix);
    let mut safe_params = Vec::new();
    let mut ffi_args = Vec::new();
    // For functions that return [*c]u8: need result_len local variable
    let needs_result_len = exp.ret_type == "[*c]u8" || exp.has_free_func;

    for param in &exp.params {
        // Skip result_len parameter: it's not part of the safe wrapper signature
        if param.name == "result_len" {
            // Still need to pass it to FFI call (as &mut result_len)
            ffi_args.push(quote! { &mut result_len });
            continue;
        }

        let param_ident = format_ident!("{}", param.name);
        let safe_ty = zig_type_to_rust_safe_type(&param.zig_type);
        safe_params.push(quote! { #param_ident: #safe_ty });
        ffi_args.push(convert_safe_to_ffi(&param.zig_type, &param_ident));
    }

    let (ret_ty, call_expr) = if needs_result_len {
        // Returns [*c]u8: need to handle result_len and free_string
        (
            quote! { String },
            quote! {
                {
                    let mut result_len: usize = 0;
                    let ptr = unsafe { super::#raw_mod::#fn_name(#(#ffi_args),*) };
                    if ptr.is_null() {
                        String::new()
                    } else {
                        // Build &[u8] slice from ptr and result_len
                        let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, result_len) };
                        let s = String::from_utf8_lossy(slice).into_owned();
                        // Call free_string to release memory
                        unsafe { super::#raw_mod::free_string(ptr, result_len) };
                        s
                    }
                }
            },
        )
    } else if exp.ret_type == "[]const u8" {
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

// ── Type conversion helpers ───────────────────────────────────────

fn zig_type_to_rust_ffi_type(zig_type: &str) -> proc_macro2::TokenStream {
    match zig_type {
        "[]const u8" => quote! { *const std::ffi::c_char },
        "i32" => quote! { i32 },
        "i64" => quote! { i64 },
        "f64" => quote! { f64 },
        "bool" => quote! { bool },
        "void" => quote! { () },
        "*usize" => quote! { *mut usize }, // result_len parameter
        _ => quote! { *mut std::ffi::c_void },
    }
}

fn zig_ret_type_to_rust_ffi(ret_type: &str) -> proc_macro2::TokenStream {
    match ret_type {
        "[]const u8" => quote! { *const std::ffi::c_char },
        "[*c]u8" => quote! { *mut std::ffi::c_char },
        "i32" => quote! { i32 },
        "i64" => quote! { i64 },
        "f64" => quote! { f64 },
        "bool" => quote! { bool },
        "void" => quote! { () },
        _ => quote! { *mut std::ffi::c_void },
    }
}

fn zig_type_to_rust_safe_type(zig_type: &str) -> proc_macro2::TokenStream {
    match zig_type {
        "[]const u8" => quote! { &str },
        "i32" => quote! { i32 },
        "i64" => quote! { i64 },
        "f64" => quote! { f64 },
        "bool" => quote! { bool },
        _ => quote! { *mut std::ffi::c_void },
    }
}

fn convert_safe_to_ffi(zig_type: &str, ident: &syn::Ident) -> proc_macro2::TokenStream {
    match zig_type {
        "[]const u8" => quote! { std::ffi::CString::new(#ident).unwrap().into_raw() },
        _ => quote! { #ident },
    }
}

fn zig_ret_type_to_rust_safe(ret_type: &str) -> proc_macro2::TokenStream {
    match ret_type {
        "[]const u8" => quote! { String },
        "i32" => quote! { i32 },
        "i64" => quote! { i64 },
        "f64" => quote! { f64 },
        "bool" => quote! { bool },
        "void" => quote! { () },
        _ => quote! { *mut std::ffi::c_void },
    }
}
