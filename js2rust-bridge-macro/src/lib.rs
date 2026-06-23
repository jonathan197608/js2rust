//! Proc-macro for generating Rust FFI bindings from js2rust C ABI export metadata.
//!
//! ## Usage
//!
//! ```rust,ignore
//! js2rust_bridge!();
//! ```
//!
//! The macro reads `js2rust.toml` from the crate root, transpiles JS to Zig,
//! writes output to `.js2zig-cache/{group}/`, and generates Rust FFI bindings.
//! The group name is derived from the file name (sanitized for Zig identifiers).
//! A minimal `build.rs` only needs `js2rust_bridge::build(false)`.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;
use std::collections::HashMap;

mod host_fn;

/// `#[host_fn]` attribute macro — eliminates unsafe C ABI plumbing from host functions.
///
/// Write a normal Rust function using SDK types (`HostStr`, `JsStr`, `JsStrField`).
/// The macro generates the `unsafe extern "C"` wrapper with correct C ABI signature.
///
/// ## Example
///
/// ```rust,ignore
/// use js2rust_bridge::sdk::{HostStr, JsStr};
///
/// #[host_fn]
/// fn host_concat(s1: HostStr, s2: HostStr) -> JsStr {
///     JsStr::new(&format!("{}{}", &s1, &s2))
/// }
/// ```
#[proc_macro_attribute]
pub fn host_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
    host_fn::host_fn_impl(attr.into(), item.into()).into()
}

// ── js2rust.toml deserialization (minimal, shared format contract) ──

#[derive(Debug, Deserialize)]
struct TomlConfig {
    project: TomlProject,
    #[serde(default)]
    host_functions: Vec<TomlHostFn>,
}

#[derive(Debug, Deserialize)]
struct TomlProject {
    js_file: String,
    #[serde(default)]
    additional_js_files: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct TomlHostFn {
    name: String,
    params: Vec<String>,
    #[serde(default)]
    returns: Option<String>,
    #[serde(default)]
    is_async: bool,
    #[serde(default)]
    async_returns: HashMap<String, String>,
}

/// Load js2rust.toml from CARGO_MANIFEST_DIR.
fn load_toml_config() -> TomlConfig {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let config_path = std::path::Path::new(&manifest_dir).join("js2rust.toml");

    let content = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        panic!(
            "js2rust_bridge: failed to read {}: {}\n\
             Create a js2rust.toml in your crate root with:\n\
             \n\
             [project]\n\
             js_file = \"js_src/main.js\"\n",
            config_path.display(),
            e
        );
    });

    toml::from_str(&content).unwrap_or_else(|e| {
        panic!(
            "js2rust_bridge: failed to parse {}: {}",
            config_path.display(),
            e
        );
    })
}

// ── C ABI export metadata (mirrors the JSON schema) ───────────────

#[derive(Debug, Deserialize)]
struct CabiExport {
    name: String,
    params: Vec<CabiParam>,
    ret_type: String,
    #[serde(default)]
    can_throw: bool,
    /// Struct return: name of the returned struct (e.g. "FetchUserResult")
    #[serde(default)]
    ret_struct_name: Option<String>,
    /// Struct return: fields of the returned struct (for generating #[repr(C)] struct)
    #[serde(default)]
    ret_struct_fields: Option<Vec<CabiStructField>>,
}

#[derive(Debug, Deserialize)]
struct CabiStructField {
    name: String,
    /// C ABI type string (for FFI)
    cabi_type: String,
}

#[derive(Debug, Deserialize)]
struct CabiParam {
    #[allow(dead_code)]
    name: String,
    zig_type: String,
}

// ── Main proc-macro entry point ───────────────────────────────────

/// Function-like proc-macro: `js2rust_bridge!()`.
///
/// Reads `js2rust.toml` from the crate root, transpiles JS to Zig,
/// and generates Rust FFI bindings.  Zero-argument — all configuration
/// lives in the TOML file.
#[proc_macro]
pub fn js2rust_bridge(input: TokenStream) -> TokenStream {
    // Accept empty input only
    let _input: proc_macro2::TokenStream = input.into();

    match generate() {
        Ok(ts) => ts,
        Err(e) => e.into(),
    }
}

// ── Transpile + generate FFI ──────────────────────────────────────

fn generate() -> Result<TokenStream, proc_macro2::TokenStream> {
    let config = load_toml_config();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

    // Derive group name from js_file stem
    let stem = std::path::Path::new(&config.project.js_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    let group = js2zig_core::analyzer::sanitize_module_name(stem);

    // Resolve core JS file path
    let js_file_path = std::path::Path::new(&manifest_dir).join(&config.project.js_file);

    // Resolve additional JS file paths
    let additional_js_paths: Vec<std::path::PathBuf> = config
        .project
        .additional_js_files
        .iter()
        .map(|f| std::path::Path::new(&manifest_dir).join(f))
        .collect();

    // Resolve cache directory
    let cache_dir = std::path::Path::new(&manifest_dir).join(".js2zig-cache");

    // Convert host function declarations to js2zig_core::HostFunction
    let mut host_functions = Vec::new();
    for hf in &config.host_functions {
        let params: Vec<_> = hf
            .params
            .iter()
            .map(|t| type_name_to_host_type(t))
            .collect();

        let return_type = hf
            .returns
            .as_deref()
            .and_then(|t| {
                if t == "void" {
                    None
                } else {
                    Some(type_name_to_host_type(t))
                }
            });

        let async_return_fields: Vec<(String, js2zig_core::HostType)> = hf
            .async_returns
            .iter()
            .map(|(name, ty)| (name.clone(), type_name_to_host_type(ty)))
            .collect();

        host_functions.push(js2zig_core::HostFunction {
            name: hf.name.clone(),
            params,
            return_type,
            is_async: hf.is_async,
            async_return_fields,
        });
    }

    // Build ProjectConfig
    let project_config = js2zig_core::ProjectConfig {
        name: group.clone(),
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
    let project_result = js2zig_core::transpile_project(&project_config).map_err(|e| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "js2rust_bridge: transpilation failed for '{}': {}",
                js_file_path.display(),
                e
            ),
        )
        .to_compile_error()
    })?;

    // Find the group result
    let group_result = project_result.groups.first().ok_or_else(|| {
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
        syn::Error::new(proc_macro2::Span::call_site(), msg).to_compile_error()
    })?;

    // Parse cabi_exports_json
    let exports: Vec<CabiExport> =
        serde_json::from_str(&group_result.cabi_exports_json).map_err(|e| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "js2rust_bridge: failed to parse cabi_exports for group '{}': {}",
                    group_result.name, e
                ),
            )
            .to_compile_error()
        })?;

    // Optionally run zig build (side effect)
    let zig_project_dir = cache_dir.join(&group);
    let lib_path = zig_project_dir
        .join("zig-out")
        .join("lib")
        .join(format!(
            "{}.lib",
            if cfg!(target_os = "windows") { &group } else { "lib" }
        ));
    let lib_exists = lib_path.exists()
        || zig_project_dir
            .join("zig-out")
            .join("lib")
            .join(format!("lib{}.a", &group))
            .exists();
    if zig_project_dir.join("build.zig").exists() && !lib_exists {
        let _ = std::process::Command::new("zig")
            .arg("build")
            .current_dir(&zig_project_dir)
            .status();
    }

    // Generate Rust FFI bindings
    let mut generated = generate_bindings(&exports, &group);

    // Generate host function stub documentation
    if !config.host_functions.is_empty() {
        if let Some(host_stubs) = generate_host_stubs(&config.host_functions, &group) {
            generated.push('\n');
            generated.push_str(&host_stubs);
        }
    }

    generated.parse::<TokenStream>().map_err(|e| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "js2rust_bridge: internal error generating bindings for '{}': {}",
                js_file_path.display(),
                e
            ),
        )
        .to_compile_error()
    })
}

// ── Type name conversion (shared) ─────────────────────────────────

fn type_name_to_host_type(name: &str) -> js2zig_core::HostType {
    match name {
        "i64" => js2zig_core::HostType::I64,
        "i32" => js2zig_core::HostType::I32,
        "f64" => js2zig_core::HostType::F64,
        "bool" => js2zig_core::HostType::Bool,
        "str" => js2zig_core::HostType::Str,
        "void" => js2zig_core::HostType::Void,
        other => panic!(
            "js2rust.toml: unknown host type '{}'. \
            Valid types: i64, i32, f64, bool, str, void",
            other
        ),
    }
}

// ── FFI bindings generation ───────────────────────────────────────

fn generate_bindings(exports: &[CabiExport], group_suffix: &str) -> String {
    let mut extern_fns = Vec::new();
    let mut safe_wrappers = Vec::new();
    let mut struct_defs = Vec::new();
    let mut needs_jsstr = false;

    let raw_mod = format_ident!("__js2rust_ffi_raw_{group_suffix}");
    let safe_mod = format_ident!("__js2rust_ffi_safe_{group_suffix}");

    // Collect struct definitions from ret_struct_name/ret_struct_fields
    for exp in exports {
        if let (Some(struct_name), Some(fields)) = (&exp.ret_struct_name, &exp.ret_struct_fields) {
            if fields.is_empty() {
                continue;
            }
            let struct_ident = format_ident!("{}", struct_name);
            let mut field_defs = Vec::new();
            for f in fields {
                let field_name = format_ident!("{}", f.name);
                let field_ty = cabi_type_to_rust_ffi(&f.cabi_type);
                field_defs.push(quote! { pub #field_name: #field_ty });
            }
            struct_defs.push(quote! {
                #[repr(C)]
                #[derive(Debug, Copy, Clone)]
                pub struct #struct_ident {
                    #(#field_defs),*
                }
            });
        }
    }

    for exp in exports {
        let fn_name = format_ident!("{}", exp.name);

        let mut extern_params = Vec::new();
        for param in &exp.params {
            let param_ident = format_ident!("{}", param.name);
            let param_ty = zig_type_to_rust_ffi_type(&param.zig_type);
            extern_params.push(quote! { #param_ident: #param_ty });
        }

        // Struct return: use out-pointer parameter
        if let Some(ret_struct_name) = &exp.ret_struct_name {
            let struct_name = format_ident!("{}", ret_struct_name);
            extern_params.push(quote! { out: *mut #struct_name });
            extern_fns.push(quote! {
                pub fn #fn_name( #(#extern_params),* );
            });
        } else {
            // Non-struct return: normal C ABI
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
        }

        let safe_wrapper = generate_safe_wrapper(exp, &fn_name, &raw_mod, group_suffix);
        safe_wrappers.push(safe_wrapper);
    }

    // Always provide js2rust_init/deinit safe wrappers
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
            #[derive(Debug, Copy, Clone)]
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
            #(#struct_defs)*
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

/// Generate host function stubs using SDK types (HostStr, JsStr, JsStrField).
///
/// Generated stubs call `HostStr::from_raw(ptr, len)` at the top and
/// `JsStr::new(&result)` for string returns — no raw pointer manipulation
/// needed in the user's business logic.
///
/// Async functions that return a struct get the struct definition with
/// `JsStrField` fields, already `#[repr(C)]`-compatible.
fn generate_host_stubs(host_fns: &[TomlHostFn], group_suffix: &str) -> Option<String> {
    if host_fns.is_empty() {
        return None;
    }

    let stub_mod = format_ident!("__js2rust_host_stubs_{group_suffix}");

    let mut async_struct_defs = Vec::new();
    let mut extern_fns = Vec::new();

    for hf in host_fns {
        let fn_sym = format_ident!("{}", hf.name);

        // Build C ABI param types: string → ptr+len, others → native
        let mut cabi_params = Vec::new();
        let mut param_conversions = Vec::new();

        for (i, param_ty) in hf.params.iter().enumerate() {
            if param_ty == "str" {
                let ptr_name = format_ident!("arg{}_ptr", i);
                let len_name = format_ident!("arg{}_len", i);
                let var_name = format_ident!("arg{}", i);
                cabi_params.push(quote! { #ptr_name: *const u8, #len_name: usize });
                param_conversions.push(quote! {
                    let #var_name = js2rust_bridge::sdk::HostStr::from_raw(#ptr_name, #len_name);
                });
            } else {
                let name = format_ident!("arg{}", i);
                let rust_ty = host_type_to_rust_cabi_ffi(param_ty);
                cabi_params.push(quote! { #name: #rust_ty });
                // No conversion needed for primitives
            }
        }

        // Return type
        let ret_ty = if !hf.async_returns.is_empty() {
            let struct_name = format_ident!("Host{}Result", pascal_case(&hf.name));
            async_struct_defs.push(generate_async_struct(
                &struct_name, &hf.async_returns,
            ));
            quote! { #struct_name }
        } else if hf.returns.as_deref() == Some("str") {
            quote! { js2rust_bridge::sdk::JsStr }
        } else if hf.returns.as_deref() == Some("void") || hf.returns.is_none() {
            quote! { () }
        } else {
            host_type_to_rust_cabi_ffi(hf.returns.as_deref().unwrap())
        };

        // Doc comment
        let doc = build_host_stub_doc(hf);

        // Build the body — param conversions + unimplemented placeholder
        let body = if hf.returns.as_deref() == Some("str") {
            // String return: show example of JsStr::new()
            let arg_refs: Vec<_> = (0..hf.params.len())
                .map(|i| {
                    let name = format_ident!("arg{}", i);
                    if hf.params[i] == "str" {
                        quote! { &#name }
                    } else {
                        quote! { #name }
                    }
                })
                .collect();
            let todo_msg = format!(
                "TODO: implement {} — replace with your logic, return sdk::JsStr::new(&result)",
                hf.name
            );
            quote! {
                #(#param_conversions)*
                let _ = (#(#arg_refs),*); // suppress unused variable warnings
                unimplemented!(#todo_msg);
            }
        } else if !hf.async_returns.is_empty() {
            // Async struct return: show example of struct construction with JsStrField::new()
            let todo_msg = format!(
                "TODO: implement {} — replace with your async logic, use JsStrField::new(&name)",
                hf.name
            );
            quote! {
                #(#param_conversions)*
                unimplemented!(#todo_msg);
            }
        } else {
            // Plain return (i64, f64, etc.)
            quote! {
                #(#param_conversions)*
                unimplemented!("TODO: implement {}", stringify!(#fn_sym));
            }
        };

        extern_fns.push(quote! {
            #[doc = #doc]
            #[allow(dead_code)]
            pub unsafe extern "C" fn #fn_sym( #(#cabi_params),* ) -> #ret_ty {
                #body
            }
        });
    }

    let output = quote! {
        /// Host function stubs generated by js2rust_bridge — uses `js2rust_bridge::sdk` types.
        ///
        /// Copy the function signatures below into your `host.rs` with `#[unsafe(no_mangle)]`
        /// and replace the `unimplemented!()` bodies with your actual business logic.
        ///
        /// The SDK handles all C ABI conversion automatically:
        /// - `HostStr::from_raw(ptr, len)` converts string params to `&str`
        /// - `JsStr::new(&s)` allocates return strings in Zig Arena
        /// - `JsStrField::new(&s)` for async struct string fields
        #[allow(dead_code, non_snake_case, clippy::not_unsafe_ptr_arg_deref)]
        mod #stub_mod {
            // SDK types are accessed via crate path, no import needed

            #(#async_struct_defs)*

            #(#extern_fns)*
        }
    };

    Some(output.to_string())
}

/// Generate an async return struct definition with SDK types.
fn generate_async_struct(
    struct_name: &syn::Ident,
    fields: &HashMap<String, String>,
) -> proc_macro2::TokenStream {
    let mut struct_fields = Vec::new();
    for (name, ty) in fields {
        let field_name = format_ident!("{}", name);
        let field_type = if ty == "str" {
            quote! { js2rust_bridge::sdk::JsStrField }
        } else {
            host_type_to_rust_cabi_ffi(ty)
        };
        struct_fields.push(quote! {
            pub #field_name: #field_type,
        });
    }

    let struct_doc = format!(
        "C ABI return struct for `{}` (generated by js2rust_bridge).\n\
         Uses `JsStrField` for string fields — allocated in Zig Arena.",
        struct_name
    );

    quote! {
        #[doc = #struct_doc]
        #[repr(C)]
        pub struct #struct_name {
            #(#struct_fields)*
        }
    }
}

/// Build a doc string for the host function stub.
fn build_host_stub_doc(hf: &TomlHostFn) -> String {
    let js_params = hf.params.join(", ");
    let js_ret = if !hf.async_returns.is_empty() {
        format!("{{ {} }}",
            hf.async_returns.iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        hf.returns.as_deref().unwrap_or("void").to_string()
    };

    let cabi_sig = hf.params.iter().enumerate().map(|(i, t)| {
        if t == "str" { format!("arg{}_ptr: *const u8, arg{}_len: usize", i, i) }
        else { format!("arg{}: {}", i, t) }
    }).collect::<Vec<_>>().join(", ");

    let ret_sig = if hf.returns.as_deref() == Some("str") {
        "JsStr".to_string()
    } else if !hf.async_returns.is_empty() {
        format!("Host{}Result", pascal_case(&hf.name))
    } else if hf.returns.as_deref() == Some("void") || hf.returns.is_none() {
        "void".to_string()
    } else {
        hf.returns.as_deref().unwrap().to_string()
    };

    let sdk_note = if hf.params.contains(&"str".to_string()) || hf.returns.as_deref() == Some("str") || !hf.async_returns.is_empty() {
        "\nSDK types used: HostStr::from_raw(ptr,len) for params, JsStr::new(&result) / JsStrField::new(&field) for returns"
    } else {
        ""
    };

    format!(
        "Host fn: {name}({js_params}) -> {js_ret}\
         \nC ABI: fn({cabi_sig}) -> {ret_sig}\
         {sdk_note}",
        name = hf.name,
        js_params = js_params,
        js_ret = js_ret,
        cabi_sig = cabi_sig,
        ret_sig = ret_sig,
        sdk_note = sdk_note,
    )
}

/// Convert snake_case to PascalCase.
fn pascal_case(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut capitalize = true;
    for ch in name.chars() {
        if ch == '_' {
            capitalize = true;
        } else if capitalize {
            result.push(ch.to_ascii_uppercase());
            capitalize = false;
        } else {
            result.push(ch);
        }
    }
    result
}

/// Convert type name to Rust C ABI FFI type.
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
    raw_mod: &syn::Ident,
    group_suffix: &str,
) -> proc_macro2::TokenStream {
    let wrapper_name = format_ident!("{}_{}", exp.name, group_suffix);
    let mut safe_params = Vec::new();
    let mut ffi_args = Vec::new();

    for param in &exp.params {
        let param_ident = format_ident!("{}", param.name);
        let safe_ty = zig_type_to_rust_safe_type(&param.zig_type);
        safe_params.push(quote! { #param_ident: #safe_ty });
        ffi_args.push(convert_safe_to_ffi(&param.zig_type, &param_ident));
    }

    // Struct return: use out-pointer
    if let Some(ref struct_name) = exp.ret_struct_name {
        let struct_ident = format_ident!("{}", struct_name);
        let ret_ty = quote! { #raw_mod::#struct_ident };
        let call_expr = quote! {
            {
                let mut result: #raw_mod::#struct_ident = unsafe { std::mem::zeroed() };
                unsafe { super::#raw_mod::#fn_name(#(#ffi_args),* , &mut result) };
                result
            }
        };
        return quote! {
            #[allow(non_snake_case)]
            pub fn #wrapper_name( #(#safe_params),* ) -> #ret_ty {
                ensure_initialized();
                #call_expr
            }
        };
    }

    // Non-struct returns
    let needs_jsstr = exp.ret_type == "StrRet";
    let has_err_out = exp.can_throw && exp.ret_type != "StrRet" && exp.ret_struct_name.is_none();

    let (ret_ty, call_expr) = if needs_jsstr {
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
    } else if has_err_out {
        let rust_ret = zig_ret_type_to_rust_safe(&exp.ret_type);
        let rust_ret_wrapped = match exp.ret_type.as_str() {
            "void" => quote! { Result<(), String> },
            _ => quote! { Result<#rust_ret, String> },
        };
        let extract_result = match exp.ret_type.as_str() {
            "void" => quote! { Ok(()) },
            _ => quote! { Ok(result) },
        };
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
        "*usize" => quote! { *mut usize },
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

/// Convert C ABI type string to Rust FFI type (for struct fields).
fn cabi_type_to_rust_ffi(cabi_type: &str) -> proc_macro2::TokenStream {
    match cabi_type {
        "i64" => quote! { i64 },
        "f64" => quote! { f64 },
        "bool" => quote! { bool },
        "StrRet" => quote! { __JsStr },
        "struct" => quote! { *mut std::ffi::c_void }, // Should not happen for struct fields
        _ => quote! { *mut std::ffi::c_void },
    }
}
