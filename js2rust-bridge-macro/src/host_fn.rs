//! `#[host_fn]` attribute macro — eliminates all unsafe C ABI plumbing.
//!
//! ## What it does
//!
//! You write a normal Rust function using SDK types (`HostStr`, `JsStr`, `JsStrField`).
//! The macro generates the `unsafe extern "C"` wrapper with correct C ABI signature
//! and delegates to your function.
//!
//! ## Example
//!
//! ```rust,ignore
//! use js2rust_bridge::sdk::{HostStr, JsStr};
//!
//! #[host_fn]
//! fn host_concat(s1: HostStr, s2: HostStr) -> JsStr {
//!     JsStr::new(&format!("{}{}", &s1, &s2))
//! }
//! ```

use syn::{ItemFn, FnArg, Pat, Type};
use quote::{quote, format_ident};
use proc_macro2::TokenStream as TokenStream2;

// ── Main macro entry point ────────────────────────────────

/// `#[host_fn]` attribute macro entry point.
///
/// Applies to a function.  Generates:
/// 1. `unsafe extern "C" fn $name(...)` — C ABI wrapper (with `#[unsafe(no_mangle)]`)
/// 2. `fn __${name}_inner(...)` — inner function (your original code)
///
/// String params declared as `HostStr` in the inner function are converted from
/// `ptr+len` C ABI params in the wrapper.
pub fn host_fn_impl(_attr: TokenStream2, item: TokenStream2) -> TokenStream2 {
    let func = syn::parse2::<ItemFn>(item)
        .unwrap_or_else(|e| panic!("#[host_fn] must be applied to a function: {}", e));

    let func_name  = &func.sig.ident;
    let inner_name = format_ident!("__{}_inner", func_name);
    let vis        = &func.vis;

    // ── Analyze params ──────────────────────────────────────
    //
    // For each param:
    //   `pat: HostStr`  →  wrapper gets `pat_ptr: *const u8, pat_len: usize`
    //                       inner gets `pat: HostStr`
    //   `pat: i64`     →  wrapper gets `pat: i64`
    //                       inner gets `pat: i64`
    //
    // Build three parallel lists:
    //   `wrap_params`  — tokens for wrapper's param list
    //   `conv_stmts`  — `let pat = HostStr::from_raw(pat_ptr, pat_len);` statements
    //   `inner_args`   — tokens to pass to `__inner(...)` call

    let mut wrap_params = Vec::<TokenStream2>::new();
    let mut conv_stmts = Vec::<TokenStream2>::new();
    let mut inner_args  = Vec::<TokenStream2>::new();

    for arg in &func.sig.inputs {
        match arg {
            FnArg::Typed(pat_type) => {
                let pat = &*pat_type.pat;   // Box<Pat>
                let ty  = &*pat_type.ty;     // Box<Type>

                if is_host_str_type(ty) {
                    // String param: ptr+len in wrapper, HostStr in inner
                    let base = pat_to_ident(pat);
                    let ptr  = format_ident!("{}_ptr", base);
                    let len  = format_ident!("{}_len", base);

                    // Wrapper param list: two entries
                    wrap_params.push(quote! { #ptr: *const u8 });
                    wrap_params.push(quote! { #len: usize });

                    // Wrapper body: convert ptr+len → HostStr
                    conv_stmts.push(quote! {
                        let #pat = unsafe {
                            js2rust_bridge::sdk::HostStr::from_raw(#ptr, #len)
                        };
                    });

                    // Inner call arg: just the identifier
                    inner_args.push(quote! { #pat });
                } else {
                    // Non-string param: pass through
                    wrap_params.push(quote! { #pat: #ty });
                    inner_args.push(quote! { #pat });
                }
            }
            FnArg::Receiver(_) => {
                panic!("#[host_fn] cannot be applied to methods (self not allowed)");
            }
        }
    }

    // ── Return type ──────────────────────────────────────────────
    //
    // `JsStr`, structs with `JsStrField`, `i64`, `bool`, `f64`, `()` are all
    // valid C ABI return types as-is (they are `#[repr(C)]` or primitives).

    let ret_ty = &func.sig.output;
    let body   = &func.block;

    // ── Build wrapper + inner function tokens ──────────────────
    //
    // We build the param list carefully to avoid trailing commas inside `()`.

    let wrap_params_tts = sep_by(wrap_params, quote! { , });
    let inner_params_tts = &func.sig.inputs;   // Punctuated — already has commas
    let inner_args_tts  = sep_by(inner_args, quote! { , });

    let wrapper = quote! {
        #[unsafe(no_mangle)]
        #vis unsafe extern "C" fn #func_name(
            #wrap_params_tts
        ) #ret_ty {
            #( #conv_stmts )*
            #inner_name( #inner_args_tts )
        }
    };

    let inner = quote! {
        #[doc(hidden)]
        #vis fn #inner_name(
            #inner_params_tts
        ) #ret_ty #body
    };

    quote! { #wrapper #inner }
}

// ── Helpers ───────────────────────────────────────────────

/// Check if a type is `HostStr` (by the last path segment).
fn is_host_str_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        tp.path.segments.last()
            .map(|seg| seg.ident == "HostStr")
            .unwrap_or(false)
    } else {
        false
    }
}

/// Extract the `Ident` from a simple pattern (`s1`, `_name`, etc.).
fn pat_to_ident(pat: &Pat) -> &syn::Ident {
    match pat {
        Pat::Ident(pat_ident) => &pat_ident.ident,
        _ => panic!("#[host_fn] param must be a simple identifier"),
    }
}

/// Join a list of `TokenStream2` with a separator, without trailing separator.
fn sep_by(items: Vec<TokenStream2>, sep: TokenStream2) -> TokenStream2 {
    if items.is_empty() {
        return TokenStream2::new();
    }
    let mut out = TokenStream2::new();
    let mut first = true;
    for item in items {
        if !first {
            out.extend(sep.clone());
        }
        out.extend(item);
        first = false;
    }
    out
}
