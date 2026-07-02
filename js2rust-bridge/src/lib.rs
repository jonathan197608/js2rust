// js2rust-bridge: Facade crate for the js2rust_bridge! proc-macro.
//
// Users only need to depend on this single crate:
//
// ```toml
// [dependencies]
// js2rust-bridge = "0.3"
// [build-dependencies]
// js2rust-bridge = "0.3"
// ```
//
// The macro transpiles JS to Zig and generates Rust FFI bindings in one step.
// Call `js2rust_bridge::build()` from your `build.rs` to transpile, compile,
// and link the Zig static libraries.  The proc-macro detects an up-to-date cache
// and skips re-transpilation automatically.
//
// Both `js2rust_bridge!()` and `build()` read from a single `js2rust.toml`
// in the crate root — no duplicated configuration.

mod config;
pub mod native_regex;
pub mod sdk;

pub use js2rust_bridge_macro::{host_fn, js2rust_bridge};

// Re-export types needed for host function configuration from js2zig-core
// so users only need `js2rust-bridge` in their `[build-dependencies]`.
pub use js2zig_core::{HostConfig, HostFunction, HostType};

// Re-export config types for programmatic use.
pub use config::{HostFnToml, Js2rustConfig, ProjectSection};

use std::path::PathBuf;

/// Build: transpile JS → Zig, compile Zig static library, and emit linker directives.
///
/// Reads configuration from `js2rust.toml` in the crate root.  Call this from
/// your `build.rs` with zero arguments:
///
/// ```rust,ignore
/// fn main() {
///     js2rust_bridge::build();
/// }
/// ```
///
/// The group name is derived automatically from the file stem of `project.js_file`.
pub fn build(force_rebuild: bool) {
    let config = Js2rustConfig::from_manifest_dir();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let cache_dir = PathBuf::from(&manifest_dir).join(".js2zig-cache");

    let js_file_path = PathBuf::from(&manifest_dir).join(&config.project.js_file);
    let additional_js_paths: Vec<PathBuf> = config
        .project
        .additional_js_files
        .iter()
        .map(|p| PathBuf::from(&manifest_dir).join(p))
        .collect();

    let group_name = config.group_name();

    let host_config = if config.host_functions.is_empty() {
        None
    } else {
        Some(build_host_config(&config))
    };

    let project_config = js2zig_core::ProjectConfig {
        name: group_name,
        js_file: js_file_path,
        additional_js_files: additional_js_paths,
        out_dir: cache_dir.clone(),
        host_config,
        force_rebuild,
        run_zig_build: true,
    };

    match js2zig_core::transpile_project(&project_config) {
        Ok(_result) => {
            link_from_cache(&cache_dir);
        }
        Err(e) => {
            panic!("js2rust_bridge::build: transpilation/zig build failed: {e}");
        }
    }
}

/// Fallback: scan `.js2zig-cache/` and emit link directives without running
/// transpilation or zig build.  Used when the proc-macro already did the work.
pub fn link() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let cache_dir = PathBuf::from(&manifest_dir).join(".js2zig-cache");
    link_from_cache(&cache_dir);
}

/// Scan `.js2zig-cache/` for compiled static libraries and emit link directives.
fn link_from_cache(cache_dir: &std::path::Path) {
    if !cache_dir.exists() {
        println!(
            "cargo:warning=js2zig: .js2zig-cache not found. \
             Run `cargo build` twice on a clean checkout."
        );
        return;
    }

    let mut found = false;
    if let Ok(entries) = std::fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let group_dir = entry.path();
            let lib_dir = group_dir.join("zig-out").join("lib");

            if lib_dir.exists()
                && let Some(group_name) = entry.file_name().to_str()
            {
                if group_name == "host.zig" || group_name == "groups.json" {
                    continue;
                }
                println!("cargo:rustc-link-search=native={}", lib_dir.display());
                println!("cargo:rustc-link-lib=static={}", group_name);
                found = true;
            }
        }
    }

    if !found {
        println!(
            "cargo:warning=js2zig: no compiled libraries found in {}. \
             Run `cargo build` twice on a clean checkout.",
            cache_dir.display()
        );
    }
}

/// Convert `Js2rustConfig.host_functions` to `HostConfig`.
fn build_host_config(config: &Js2rustConfig) -> HostConfig {
    let functions: Vec<HostFunction> = config
        .host_functions
        .iter()
        .map(|hf| {
            let params: Vec<HostType> = hf
                .params
                .iter()
                .map(|t| type_name_to_host_type(t))
                .collect();

            let return_type = hf.returns.as_deref().and_then(|t| {
                if t == "void" {
                    None
                } else {
                    Some(type_name_to_host_type(t))
                }
            });

            let async_return_fields: Vec<(String, HostType)> = hf
                .async_returns
                .iter()
                .map(|(name, ty)| (name.clone(), type_name_to_host_type(ty)))
                .collect();

            HostFunction {
                name: hf.name.clone(),
                params,
                return_type,
                is_async: hf.is_async,
                async_return_fields,
            }
        })
        .collect();

    HostConfig { functions }
}

fn type_name_to_host_type(name: &str) -> HostType {
    match name {
        "i64" => HostType::I64,
        "i32" => HostType::I32,
        "f64" => HostType::F64,
        "bool" => HostType::Bool,
        "str" => HostType::Str,
        "void" => HostType::Void,
        other => panic!(
            "js2rust.toml: unknown host type '{}'. Valid types: i64, i32, f64, bool, str, void",
            other
        ),
    }
}
