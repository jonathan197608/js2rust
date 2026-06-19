// js2rust-bridge: Facade crate for the js2rust_bridge! proc-macro.
//
// Users only need to depend on this single crate:
//
// ```toml
// [dependencies]
// js2rust-bridge = "0.1"
// [build-dependencies]
// js2rust-bridge = "0.1"
// ```
//
// The macro transpiles JS to Zig and generates Rust FFI bindings in one step.
// Call `js2rust_bridge::link()` from your `build.rs` to link the compiled
// static libraries.

pub use js2rust_bridge_macro::js2rust_bridge;

/// Scan `.js2zig-cache/` and emit `cargo:rustc-link-*` directives for each
/// compiled static library.
///
/// Call this from your `build.rs`:
///
/// ```rust,ignore
/// fn main() {
///     js2rust_bridge::link();
/// }
/// ```
pub fn link() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let cache_dir = std::path::Path::new(&manifest_dir).join(".js2zig-cache");

    println!("cargo:rerun-if-changed=.js2zig-cache");

    if !cache_dir.exists() {
        println!(
            "cargo:warning=js2zig: .js2zig-cache not found. \
             Run `cargo build` twice on a clean checkout."
        );
        return;
    }

    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            let group_dir = entry.path();
            let lib_dir = group_dir.join("zig-out").join("lib");

            if lib_dir.exists()
                && let Some(group_name) = entry.file_name().to_str()
            {
                println!("cargo:rustc-link-search=native={}", lib_dir.display());
                println!("cargo:rustc-link-lib=static={}", group_name);
            }
        }
    }
}
