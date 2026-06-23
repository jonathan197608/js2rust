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
    braced, parenthesized,
    parse::{Parse, ParseStream},
    Ident, LitStr, Token,
};

// ── C ABI export metadata (mirrors the JSON schema) ───────────────

#[derive(Debug, Deserialize)]
struct CabiExport {
    name: String,
    params: Vec<CabiParam>,
    ret_type: String,
    #[serde(default)]
    can_throw: bool,
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
    /// Additional JS file paths (multi-root mode).
    additional_js_files: Vec<String>,
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

        // After the primary JS file: additional JS files or host functions
        // (all comma-separated).  Additional JS files are LitStr; host functions
        // start with an optional `async` keyword followed by an Ident.
        let mut additional_js_files = Vec::new();
        let mut host_fns = Vec::new();
        loop {
            if !input.peek(Token![,]) {
                break;
            }
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            // Decide: LitStr → additional JS file; async/Ident → host function
            if input.peek(LitStr) {
                let additional: LitStr = input.parse()?;
                additional_js_files.push(additional.value());
                continue;
            }
            // Must be a host function declaration

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
            additional_js_files,
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
        other => Err(format!(
            "js2rust_bridge: unknown host type '{}'. \
            Valid types: i64, i32, f64, bool, str, void",
            other
        )),
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
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

    // Resolve core JS file path
    let js_file_path = std::path::Path::new(&manifest_dir).join(&input.js_file);

    // Resolve cache directory for Zig output
    let cache_dir = std::path::Path::new(&manifest_dir).join(".js2zig-cache");

    // Convert host function declarations to js2zig_core::HostFunction
    let mut host_functions = Vec::new();
    for hf in &input.host_fns {
        let params: Result<Vec<_>, _> = hf
            .params
            .iter()
            .map(|t| type_name_to_host_type(t))
            .collect();
        let params = match params {
            Ok(p) => p,
            Err(e) => {
                return syn::Error::new(proc_macro2::Span::call_site(), e)
                    .to_compile_error()
                    .into()
            }
        };

        let return_type = match type_name_to_host_type(&hf.return_type) {
            Ok(js2zig_core::HostType::Void) => None,
            Ok(t) => Some(t),
            Err(e) => {
                return syn::Error::new(proc_macro2::Span::call_site(), e)
                    .to_compile_error()
                    .into()
            }
        };

        // Convert async return fields
        let async_return_fields: Result<Vec<_>, _> = hf
            .async_return_fields
            .iter()
            .map(|(name, ty)| type_name_to_host_type(ty).map(|t| (name.clone(), t)))
            .collect();
        let async_return_fields = match async_return_fields {
            Ok(v) => v,
            Err(e) => {
                return syn::Error::new(proc_macro2::Span::call_site(), e)
                    .to_compile_error()
                    .into()
            }
        };

        host_functions.push(js2zig_core::HostFunction {
            name: hf.name.clone(),
            params,
            return_type,
            is_async: hf.is_async,
            async_return_fields,
        });
    }

    // Resolve additional JS file paths (multi-root mode)
    let additional_js_paths: Vec<std::path::PathBuf> = input
        .additional_js_files
        .iter()
        .map(|f| std::path::Path::new(&manifest_dir).join(f))
        .collect();

    // Build ProjectConfig
    let config = js2zig_core::ProjectConfig {
        name: input.group.clone(),
        js_file: js_file_path.clone(),
        additional_js_files: additional_js_paths,
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
                format!(
                    "js2rust_bridge: transpilation failed for '{}': {}",
                    js_file_path.display(),
                    e
                ),
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
            let mut msg = format!(
                "js2rust_bridge: no groups found in transpilation result for '{}'.",
                js_file_path.display()
            );
            if !project_result.diagnostics.is_empty() {
                msg.push_str("\n\nTranspilation diagnostics:");
                for diag in &project_result.diagnostics {
                    msg.push_str(&format!("\n  - {}", diag));
                }
            }
            return syn::Error::new(proc_macro2::Span::call_site(), msg)
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
                format!(
                    "js2rust_bridge: failed to parse cabi_exports for group '{}': {}",
                    group_result.name, e
                ),
            )
            .to_compile_error()
            .into();
        }
    };

    // Optionally run zig build (side effect — generates static library for linking).
    // Only run if the .lib doesn't already exist (build.rs may have built it first).
    let zig_project_dir = cache_dir.join(&input.group);
    let lib_path = zig_project_dir
        .join("zig-out")
        .join("lib")
        .join(format!(
            "{}.lib",
            if cfg!(target_os = "windows") {
                &input.group
            } else {
                "lib"
            }
        ));
    let lib_exists = lib_path.exists()
        || zig_project_dir
            .join("zig-out")
            .join("lib")
            .join(format!("lib{}.a", &input.group))
            .exists();
    if zig_project_dir.join("build.zig").exists() && !lib_exists {
        let _ = std::process::Command::new("zig")
            .arg("build")
            .current_dir(&zig_project_dir)
            .status();
    }

    // Generate Rust FFI bindings from cabi exports
    let mut generated = generate_bindings(&exports, &input.group);

    // Generate host function stub documentation (zero-copy C ABI signatures)
    if !input.host_fns.is_empty() {
        if let Some(host_stubs) = generate_host_stubs(&input.host_fns, &input.group) {
            generated.push('\n');
            generated.push_str(&host_stubs);
        }
    }

    match generated.parse::<TokenStream>() {
        Ok(ts) => ts,
        Err(e) => syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "js2rust_bridge: internal error generating bindings for '{}': {}",
                js_file_path.display(),
                e
            ),
        )
        .to_compile_error()
        .into(),
    }
}

// ── FFI bindings generation ───────────────────────────────────────

fn generate_bindings(exports: &[CabiExport], group_suffix: &str) -> String {
    let mut extern_fns = Vec::new();
    let mut safe_wrappers = Vec::new();
    let mut needs_jsstr = false;

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

        // For non-StrRet can_throw functions: add err_out parameter
        let has_err_out = exp.can_throw && exp.ret_type != "StrRet";
        if has_err_out {
            extern_params.push(quote! { err_out: *mut *const std::ffi::c_char });
        }

        let ret_ty = zig_ret_type_to_rust_ffi(&exp.ret_type);

        extern_fns.push(quote! {
            pub fn #fn_name( #(#extern_params),* ) -> #ret_ty;
        });

        if exp.ret_type == "StrRet" {
            needs_jsstr = true;
        }

        let safe_wrapper =
            generate_safe_wrapper(exp, &fn_name, &free_fn_name, &raw_mod, group_suffix, has_err_out);
        safe_wrappers.push(safe_wrapper);
    }

    // Always provide js2rust_init/deinit safe wrappers (C ABI exports from lib.zig)
    // Use Once to ensure js2rust_init() is called exactly once before any FFI call.
    let runtime_init = quote! {
        use std::sync::Once;

        static INIT: Once = Once::new();

        /// Initialize the Zig runtime (allocator + Io for async functions).
        /// Call this before any async export function.
        /// Safe to call multiple times (uses Once internally).
        pub fn js2rust_init() {
            extern "C" {
                #[link_name = "js2rust_init"]
                fn _js2rust_init();
            }
            INIT.call_once(|| {
                unsafe { _js2rust_init() };
            });
        }

        /// Ensure the Zig runtime is initialized (calls js2rust_init() exactly once).
        /// Called automatically by all generated wrapper functions.
        fn ensure_initialized() {
            js2rust_init();
        }

        /// Release Zig runtime resources.
        pub fn js2rust_deinit() {
            extern "C" {
                #[link_name = "js2rust_deinit"]
                fn _js2rust_deinit();
            }
            unsafe { _js2rust_deinit() };
        }

        /// Reset the Zig arena allocator (free all allocated memory, keep allocator active).
        /// Call this periodically to prevent excessive memory usage.
        /// Thread-safe: uses atomic spinlock internally (in Zig code).
        /// After reset, all previously returned pointers from Zig functions become invalid.
        /// Make sure no Zig function is running when calling this.
        pub fn js2rust_reset() {
            extern "C" {
                #[link_name = "js2rust_reset"]
                fn _js2rust_reset();
            }
            unsafe { _js2rust_reset() };
        }
    };
    safe_wrappers.push(runtime_init);

    // Conditionally define __JsStr in raw_mod for StrRet-returning functions.
    let jsstr_def = if needs_jsstr {
        quote! {
            #[repr(C)]
            pub struct __JsStr { pub ptr: *const u8, pub len: isize }
        }
    } else {
        quote! {}
    };

    let output = quote! {
        #[allow(non_snake_case)]
        #[allow(dead_code)]
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        mod #raw_mod {
            #jsstr_def
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

/// Generate host function stub documentation for the Rust side.
///
/// For each host function declared in the macro, this generates:
/// 1. `__JsStr` struct definition (if any str-returning host fn)
/// 2. `js_allocator_alloc` extern declaration (for zero-copy string returns)
/// 3. Doc comments showing the exact zero-copy C ABI signature
///
/// Users should copy these signatures into their `host.rs` and replace
/// `unimplemented!()` with actual logic.
fn generate_host_stubs(host_fns: &[HostFnDecl], group_suffix: &str) -> Option<String> {
    if host_fns.is_empty() {
        return None;
    }

    let stub_mod = format_ident!("__js2rust_host_stubs_{group_suffix}");

    let mut has_str_ret = false;

    // Build the extern function declarations
    let mut extern_fns = Vec::new();

    for hf in host_fns {
        let fn_sym = format_ident!("{}", hf.name);

        // Build C ABI param types: string → ptr+len pair
        let mut cabi_params = Vec::new();
        for (i, param_ty) in hf.params.iter().enumerate() {
            if param_ty == "str" {
                let ptr_name = format_ident!("arg{}_ptr", i);
                let len_name = format_ident!("arg{}_len", i);
                cabi_params.push(quote! { #ptr_name: *const u8, #len_name: usize });
            } else {
                let name = format_ident!("arg{}", i);
                let rust_ty = host_type_to_rust_cabi_ffi(param_ty);
                cabi_params.push(quote! { #name: #rust_ty });
            }
        }

        // Return type
        let ret_ty = if !hf.async_return_fields.is_empty() {
            // Async struct return — user defines struct in host.rs, stub uses () 
            quote! { () }
        } else if hf.return_type == "str" {
            has_str_ret = true;
            quote! { __JsStr }
        } else if hf.return_type == "void" {
            quote! { () }
        } else {
            host_type_to_rust_cabi_ffi(&hf.return_type)
        };

        // Doc comment with JS-level signature and C ABI signature
        let js_params = hf.params.join(", ");
        let js_ret = if hf.return_type == "void" {
            "void".to_string()
        } else {
            hf.return_type.clone()
        };
        let doc = format!(
            "Host fn: {name}({js_params}) -> {js_ret}\
             \nZero-copy C ABI: fn({cabi_sig}) -> {ret_sig}\
             \nString params: pass .ptr and .len (in Zig Arena, no copy)\
             \nString returns: allocate via js_allocator_alloc(), return __JsStr",
            name = hf.name,
            js_params = js_params,
            js_ret = js_ret,
            cabi_sig = hf.params.iter().enumerate().map(|(i, t)| {
                if t == "str" { format!("arg{}_ptr: *const u8, arg{}_len: usize", i, i) }
                else { format!("arg{}: {}", i, t) }
            }).collect::<Vec<_>>().join(", "),
            ret_sig = if hf.return_type == "str" { "__JsStr" }
                else if hf.return_type == "void" { "void" }
                else { &hf.return_type },
        );

        extern_fns.push(quote! {
            #[doc = #doc]
            #[allow(dead_code)]
            pub unsafe extern "C" fn #fn_sym( #(#cabi_params),* ) -> #ret_ty {
                unimplemented!("TODO: implement {}", stringify!(#fn_sym))
            }
        });
    }

    // __JsStr definition (only if needed by host fns or exports)
    let jsstr_def = if has_str_ret {
        quote! {
            /// C ABI return type for zero-copy strings (memory in Zig Arena).
            /// ptr+len pair with sign-bit convention:
            ///   len >= 0 → normal string of that length
            ///   len < 0  → panic/error, |len| bytes contain error name
            #[repr(C)]
            pub struct __JsStr {
                pub ptr: *const u8,
                pub len: isize,
            }
        }
    } else {
        quote! {}
    };

    // js_allocator_alloc declaration (only if any host fn returns string)
    let alloc_decl = if has_str_ret {
        quote! {
            extern "C" {
                /// Allocate memory in Zig's Arena for zero-copy string returns.
                /// Memory is managed by Zig's dual-arena allocator — no free needed.
                pub fn js_allocator_alloc(size: usize) -> *mut u8;
            }
        }
    } else {
        quote! {}
    };

    let output = quote! {
        /// Host function stubs generated by js2rust_bridge.
        /// Copy the signatures below into your `host.rs` with `#[unsafe(no_mangle)]`
        /// and replace the `unimplemented!()` bodies with your actual logic.
        #[allow(dead_code, non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
        mod #stub_mod {
            #jsstr_def

            #alloc_decl

            #(#extern_fns)*
        }
    };

    Some(output.to_string())
}

/// Convert macro-level host type name to Rust C ABI FFI type.
fn host_type_to_rust_cabi_ffi(type_name: &str) -> proc_macro2::TokenStream {
    match type_name {
        "i64" => quote! { i64 },
        "i32" => quote! { i32 },
        "f64" => quote! { f64 },
        "bool" => quote! { bool },
        "void" => quote! { () },
        _ => quote! { *mut std::ffi::c_void },
    }
}

fn generate_safe_wrapper(
    exp: &CabiExport,
    fn_name: &syn::Ident,
    free_fn_name: &syn::Ident,
    raw_mod: &syn::Ident,
    group_suffix: &str,
    has_err_out: bool,
) -> proc_macro2::TokenStream {
    let wrapper_name = format_ident!("{}_{}", exp.name, group_suffix);
    let mut safe_params = Vec::new();
    let mut ffi_args = Vec::new();
    // For functions that return StrRet: need JsStr struct
    let needs_jsstr = exp.ret_type == "StrRet";

    for param in &exp.params {
        let param_ident = format_ident!("{}", param.name);
        let safe_ty = zig_type_to_rust_safe_type(&param.zig_type);
        safe_params.push(quote! { #param_ident: #safe_ty });
        ffi_args.push(convert_safe_to_ffi(&param.zig_type, &param_ident));
    }

    let (ret_ty, call_expr) = if needs_jsstr {
        // Returns StrRet (extern struct { ptr: *const u8, len: isize }).
        // Sign-bit convention: len >= 0 → normal string; len < 0 → async panic.
        // Rust safe wrapper converts to Result<String, String>.
        (
            quote! { Result<String, String> },
            quote! {
                {
                    let ret: #raw_mod::__JsStr = unsafe { super::#raw_mod::#fn_name(#(#ffi_args),*) };
                    if ret.len < 0 {
                        let err_len = (-ret.len) as usize;
                        let err_msg = if err_len > 0 && !ret.ptr.is_null() {
                            let slice = unsafe { std::slice::from_raw_parts(ret.ptr, err_len) };
                            String::from_utf8_lossy(slice).into_owned()
                        } else {
                            "unknown async error".to_string()
                        };
                        return Err(err_msg);
                    }
                    if ret.ptr.is_null() {
                        Ok(String::new())
                    } else {
                        let len = ret.len as usize;
                        let slice = unsafe { std::slice::from_raw_parts(ret.ptr, len) };
                        Ok(String::from_utf8_lossy(slice).into_owned())
                    }
                }
            },
        )
    } else if exp.ret_type == "[]const u8" {
        if has_err_out {
            // Non-StrRet string return with throw: Result<String, String> via err_out
            let mut all_ffi_args: Vec<proc_macro2::TokenStream> = ffi_args.clone();
            all_ffi_args.push(quote! { &mut err_ptr });
            (
                quote! { Result<String, String> },
                quote! {
                    {
                        let mut err_ptr: *const std::ffi::c_char = std::ptr::null();
                        let ptr = unsafe { super::#raw_mod::#fn_name(#(#all_ffi_args),*) };
                        if !err_ptr.is_null() {
                            let err_msg = unsafe { std::ffi::CStr::from_ptr(err_ptr).to_string_lossy().into_owned() };
                            return Err(err_msg);
                        }
                        if ptr.is_null() {
                            Ok(String::new())
                        } else {
                            let s = unsafe {
                                std::ffi::CStr::from_ptr(ptr)
                                    .to_string_lossy()
                                    .into_owned()
                            };
                            unsafe { super::#raw_mod::#free_fn_name(ptr as *mut std::ffi::c_void) };
                            Ok(s)
                        }
                    }
                },
            )
        } else {
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
        }
    } else if has_err_out {
        // Non-StrRet can_throw: Result<T, String> via err_out
        let rust_ret = zig_ret_type_to_rust_safe(&exp.ret_type);
        let rust_ret_wrapped = match exp.ret_type.as_str() {
            "void" => quote! { Result<(), String> },
            _ => quote! { Result<#rust_ret, String> },
        };
        let extract_result = match exp.ret_type.as_str() {
            "void" => quote! { Ok(()) },
            _ => quote! { Ok(result) },
        };
        // Build all ffi args including err_out
        let mut all_ffi_args: Vec<proc_macro2::TokenStream> = ffi_args.clone();
        all_ffi_args.push(quote! { &mut err_ptr });
        (
            rust_ret_wrapped,
            quote! {
                {
                    let mut err_ptr: *const std::ffi::c_char = std::ptr::null();
                    let result = unsafe { super::#raw_mod::#fn_name(#(#all_ffi_args),*) };
                    if !err_ptr.is_null() {
                        let err_msg = unsafe { std::ffi::CStr::from_ptr(err_ptr).to_string_lossy().into_owned() };
                        return Err(err_msg);
                    }
                    #extract_result
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
            // Ensure Zig runtime (allocator) is initialized before calling FFI
            ensure_initialized();
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
        "StrRet" => quote! { __JsStr },
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
