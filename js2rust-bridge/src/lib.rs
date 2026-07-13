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
//
// `build()` emits `cargo:rerun-if-changed` directives for every JS source
// file, the `js2rust.toml` config, and the build cache, so Cargo automatically
// re-runs the build script when any input changes.

mod config;
#[cfg(feature = "icu")]
pub mod native_icu;
#[cfg(feature = "regex")]
pub mod native_regex;
pub mod sdk;

pub use js2rust_bridge_macro::{host_fn, js2rust_bridge};

// Re-export types needed for host function configuration from js2zig-core
// so users only need `js2rust-bridge` in their `[build-dependencies]`.
pub use js2zig_core::{HostConfig, HostFunction, HostType};

// Re-export config types for programmatic use.
pub use config::{BuildSection, HostFnToml, Js2rustConfig, ProjectSection};

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
/// The project name is derived automatically from the file stem of the first
/// entry in `project.js_files`. Build behavior (force_rebuild, run_zig_build)
/// is read from the `[build]` section of `js2rust.toml`.
pub fn build() {
    let config = Js2rustConfig::from_manifest_dir();
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo");
    let cache_dir = PathBuf::from(&manifest_dir).join(".js2zig-cache");

    // All JS files are resolved from config (entry + additional roots)
    let js_file_paths: Vec<PathBuf> = config
        .project
        .js_files
        .iter()
        .map(|p| PathBuf::from(&manifest_dir).join(p))
        .collect();

    // js_files must not be empty
    if js_file_paths.is_empty() {
        panic!("js2rust_bridge::build: project.js_files is empty in js2rust.toml");
    }

    // Emit cargo:rerun-if-changed for explicitly listed JS files and config.
    // Note: js2zig-core::transpile_project() additionally emits rerun-if-changed
    // for every JS file it discovers via import analysis (including transitive
    // dependencies like helpers.js).  Those directives take effect in subsequent
    // builds because Cargo stores all rerun-if-changed paths from the previous
    // build script run.
    for p in &js_file_paths {
        println!("cargo:rerun-if-changed={}", p.display());
    }
    let config_toml = PathBuf::from(&manifest_dir).join("js2rust.toml");
    println!("cargo:rerun-if-changed={}", config_toml.display());
    // Watch the diagnostics tracking file so Cargo re-runs the build script
    // when it is first created or changes.  This ensures the build script
    // produces a CLEAN output (no cargo:warning) on the next run after the
    // initial diagnostic emission, preventing Cargo from replaying stale
    // warnings on cached builds.
    let diagnostics_file = cache_dir.join(".last_emitted_diagnostics.json");
    println!("cargo:rerun-if-changed={}", diagnostics_file.display());

    let (entry_file, additional_roots) = {
        let mut paths = js_file_paths;
        let entry = paths.remove(0);
        (entry, paths)
    };

    let host_config = config.to_host_config();

    // Determine Zig optimization level: TOML override > Cargo PROFILE auto-detect.
    let zig_optimize = config.build.zig_optimize.clone().unwrap_or_else(|| {
        let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
        if profile == "release" {
            "ReleaseSafe".into()
        } else {
            "Debug".into()
        }
    });

    let project_config = js2zig_core::ProjectConfig {
        entry_file,
        additional_roots,
        out_dir: cache_dir.clone(),
        host_config,
        force_rebuild: config.build.force_rebuild,
        run_zig_build: config.build.run_zig_build,
        zig_optimize: Some(zig_optimize),
        is_build_script: true, // build.rs context — show progress + emit rerun-if-changed
    };
    match js2zig_core::transpile_project(&project_config) {
        Ok(result) => {
            // Emit compile errors as cargo:warning — but only when they CHANGE.
            // Cargo replays cargo:warning from the previous build script run,
            // so if we always emit them the user sees the same warnings on every
            // build.  By tracking the last-emitted set and only emitting new/
            // changed diagnostics, we produce a clean build script output once
            // the diagnostics stabilize, and Cargo will replay that clean output
            // on subsequent cached builds.
            let compile_errors: Vec<&str> = result
                .diagnostics
                .iter()
                .filter(|d| d.contains("COMPILE_ERROR"))
                .map(|s| s.as_str())
                .collect();

            let last_emitted_path = cache_dir.join(".last_emitted_diagnostics.json");
            let last_emitted: Vec<String> = std::fs::read_to_string(&last_emitted_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            // Only emit warnings for diagnostics that differ from the last emission
            if !compile_errors.iter().eq(last_emitted.iter()) {
                for diag in &compile_errors {
                    println!("cargo:warning=js2zig: {diag}");
                }
                // Persist current set so the next build can compare
                if let Ok(json) = serde_json::to_string_pretty(&compile_errors) {
                    let _ = std::fs::write(&last_emitted_path, json);
                }
            }

            link_from_cache(&cache_dir);
        }
        Err(e) => {
            panic!("js2rust_bridge::build: transpilation/zig build failed: {e}");
        }
    }
}

/// Fallback: scan `.js2zig-cache/` and emit link directives without running
/// transpilation or zig build.  Used when the proc-macro already did the work.
///
/// Emits `cargo:rerun-if-changed` for the cache directory so Cargo re-runs
/// the build script when the compiled libraries change.
pub fn link() {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by Cargo");
    let cache_dir = PathBuf::from(&manifest_dir).join(".js2zig-cache");

    // Re-run when compiled libraries appear or change.
    if cache_dir.exists() {
        println!("cargo:rerun-if-changed={}", cache_dir.display());
    }

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
            let project_dir = entry.path();
            let lib_dir = project_dir.join("zig-out").join("lib");

            if lib_dir.exists()
                && let Some(dir_name) = entry.file_name().to_str()
            {
                if dir_name == "host_regex.zig"
                    || dir_name == "host_icu.zig"
                    || dir_name == "host_regex_stubs.zig"
                    || dir_name == "host_icu_stubs.zig"
                {
                    continue;
                }
                println!("cargo:rustc-link-search=native={}", lib_dir.display());
                println!("cargo:rustc-link-lib=static={}", dir_name);
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
