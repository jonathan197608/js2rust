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
// Call `js2rust_bridge::build(...)` from your `build.rs` to transpile, compile,
// and link the Zig static libraries.  The proc-macro detects an up-to-date cache
// and skips re-transpilation automatically.

pub use js2rust_bridge_macro::js2rust_bridge;

// Re-export types needed for BuildConfig from js2zig-core so users only
// need `js2rust-bridge` in their `[build-dependencies]`.
pub use js2zig_core::{HostConfig, HostFunction, HostType};

use std::path::PathBuf;

/// Simplified build configuration for `build.rs`.
/// Mirrors `js2zig_core::ProjectConfig` without exposing the full API.
pub struct BuildConfig {
    /// Project name (also used as Zig library name).
    pub name: String,
    /// Core JS source file path (the entry point; its imports are pulled in transitively).
    /// Relative to CARGO_MANIFEST_DIR.
    pub js_file: String,
    /// Additional core JS files (multi-root: all roots + their transitive deps → one group).
    pub additional_js_files: Vec<String>,
    /// Host function declarations (for projects that use `host_*` calls from JS).
    /// `None` if this project doesn't use host functions.
    pub host_functions: Option<HostConfig>,
    /// Force rebuild (ignore incremental cache).
    pub force_rebuild: bool,
}

/// Transpile JS → Zig, build the Zig static library, and emit `cargo:rustc-link-*`
/// directives for the linker.
///
/// Call this from your `build.rs`.  On the first build after a clean checkout
/// this does the heavy lifting (transpile + `zig build`).  On subsequent builds
/// it detects the up-to-date cache and is a near-no-op.
///
/// ```rust,ignore
/// fn main() {
///     js2rust_bridge::build(js2rust_bridge::BuildConfig {
///         name: "main".into(),
///         js_file: "js_src/main.js".into(),
///         additional_js_files: vec![],
///         host_functions: Default::default(),
///         force_rebuild: false,
///     });
/// }
/// ```
pub fn build(config: BuildConfig) {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let cache_dir = PathBuf::from(&manifest_dir).join(".js2zig-cache");

    // Convert BuildConfig → ProjectConfig
    let js_file_path = PathBuf::from(&manifest_dir).join(&config.js_file);
    let additional_js_paths: Vec<PathBuf> = config
        .additional_js_files
        .iter()
        .map(|p| PathBuf::from(&manifest_dir).join(p))
        .collect();

    let project_config = js2zig_core::ProjectConfig {
        name: config.name,
        js_file: js_file_path,
        additional_js_files: additional_js_paths,
        out_dir: cache_dir.clone(),
        host_config: config.host_functions,
        force_rebuild: config.force_rebuild,
        run_zig_build: true, // build.rs runs zig build
    };

    // Transpile JS → Zig (generates .js2zig-cache/{name}/)
    match js2zig_core::transpile_project(&project_config) {
        Ok(_result) => {
            // Emit link directives for each group's compiled static library.
            link_from_cache(&cache_dir);
        }
        Err(e) => {
            // If transpilation fails, still try to link existing cache (e.g. from
            // a previous successful build).
            eprintln!("js2rust_bridge::build: transpilation error: {}", e);
            link_from_cache(&cache_dir);
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
                && let Some(group_name) = entry.file_name().to_str() {
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
