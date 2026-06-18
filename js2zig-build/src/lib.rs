// js2zig-build: build.rs helper for js2zig-core.
//
// Usage in external project's `build.rs`:
//
// ```rust
// fn main() {
//     js2zig_build::transpile("js_src");
// }
// ```
//
// This will:
// 1. Read JS files from `js_src/` (relative to CARGO_MANIFEST_DIR)
// 2. Transpile to Zig, output to `$OUT_DIR/js2zig/` (set by Cargo)
// 3. Write `cabi_exports.json` for each group
//
// Then in your Rust code, use `js2rust-bridge` to generate FFI bindings.

use std::path::Path;

/// Transpile JS directory to Zig, output to `$OUT_DIR/js2zig/`.
///
/// `js_dir` is relative to `CARGO_MANIFEST_DIR`.
/// Output is written to `$OUT_DIR/js2zig/` (where `$OUT_DIR` is set by Cargo).
///
/// # Panics
/// Panics if `OUT_DIR` is not set (i.e., not running under `cargo build`).
pub fn transpile(js_dir: &str) {
    let out_dir = std::env::var("OUT_DIR").expect(
        "js2zig-build: OUT_DIR not set. Are you running under `cargo build`?",
    );
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect(
        "js2zig-build: CARGO_MANIFEST_DIR not set",
    );

    let js_path = Path::new(&manifest_dir).join(js_dir);
    let out_path = Path::new(&out_dir).join("js2zig");

    let config = js2zig_core::ProjectConfig {
        name: std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "js2zig_lib".into()),
        js_dir: js_path,
        out_dir: out_path.clone(),  // Clone before moving
        host_config: None,
        force_rebuild: false,
        run_zig_build: false, // Don't run zig build during cargo build
    };

    match js2zig_core::transpile_project(&config) {
        Ok(_result) => {
            // Output files are written to $OUT_DIR/js2zig/
            // The js2rust-bridge-macro will read $OUT_DIR/js2zig/{group}/cabi_exports.json
        }
        Err(e) => {
            eprintln!("js2zig-build error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Transpile with explicit configuration.
pub fn transpile_with_config(config: &js2zig_core::ProjectConfig) {
    match js2zig_core::transpile_project(config) {
        Ok(_result) => {}
        Err(e) => {
            eprintln!("js2zig-build error: {}", e);
            std::process::exit(1);
        }
    }
}
