// build.rs for js2rust-bridge crate
// Reads intermediate artifacts produced by `js2rustc` (out/cabi_exports.json),
// runs `zig build` on the already-generated Zig project (out/js2rust/),
// generates Rust FFI bindings from the JSON metadata, and informs Cargo about linking.
//
// NOTE: Run `cargo run -p js2rustc` first to generate the artifacts in out/.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let ws_root = manifest_dir.parent().unwrap();

    let artifacts_dir = ws_root.join("out");
    let zig_project_dir = artifacts_dir.join("js2rust");

    // Re-run triggers
    println!(
        "cargo:rerun-if-changed={}",
        artifacts_dir.join("cabi_exports.json").to_string_lossy()
    );
    println!(
        "cargo:rerun-if-changed={}",
        zig_project_dir.join("src").to_string_lossy()
    );

    // === Check that core has been run ===
    let cabi_json_path = artifacts_dir.join("cabi_exports.json");
    if !cabi_json_path.exists() {
        panic!(
            "{} not found.\n\
             Run `cargo run -p js2rustc` first to generate the Zig project and metadata.",
            cabi_json_path.display()
        );
    }

    // === Read C ABI exports metadata ===
    let cabi_json_str = fs::read_to_string(&cabi_json_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", cabi_json_path.display(), e));
    let cabi_exports: Vec<serde_json::Value> = serde_json::from_str(&cabi_json_str)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", cabi_json_path.display(), e));

    // === Phase: Build Zig static library ===
    if !zig_project_dir.exists() {
        panic!(
            "{} not found.\n\
             Run `cargo run -p js2rustc` first to generate the Zig project.",
            zig_project_dir.display()
        );
    }

    let status = Command::new("zig")
        .args(["build", "-Doptimize=ReleaseSafe"])
        .current_dir(&zig_project_dir)
        .status()
        .expect("Failed to run zig build; is Zig installed?");
    if !status.success() {
        let _ = Command::new("zig")
            .args(["build", "test"])
            .current_dir(&zig_project_dir)
            .status();
        panic!("zig build failed in {}", zig_project_dir.display());
    }

    // === Find the compiled static library ===
    let lib_dir = find_static_lib_dir(&zig_project_dir);
    let _lib_path = lib_dir.join("js2rust.lib");

    println!(
        "cargo:rustc-link-search=native={}",
        lib_dir.to_string_lossy()
    );
    println!("cargo:rustc-link-lib=static=js2rust");

    // NT API symbols (ntdll.lib provides LdrRegisterDllNotification etc.)
    println!("cargo:rustc-link-lib=ntdll");

    // === Generate Rust FFI bindings ===
    let bindings_path = out_dir.join("ffi_bindings.rs");
    generate_ffi_bindings(&cabi_exports, &bindings_path);
}

/// Locate the directory containing the compiled static library.
fn find_static_lib_dir(zig_project_dir: &Path) -> PathBuf {
    let zig_out = zig_project_dir.join("zig-out").join("lib");
    if zig_out.is_dir() && has_static_lib(&zig_out) {
        return zig_out;
    }
    let zig_cache = zig_project_dir.join(".zig-cache").join("lib");
    if zig_cache.is_dir() && has_static_lib(&zig_cache) {
        return zig_cache;
    }
    search_for_static_lib(zig_project_dir)
        .unwrap_or_else(|| panic!("Could not find compiled static library js2rust.lib"))
}

fn has_static_lib(dir: &Path) -> bool {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy() == "js2rust.lib" {
                return true;
            }
        }
    }
    false
}

fn search_for_static_lib(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(result) = search_for_static_lib(&path) {
                    return Some(result);
                }
            } else if path.file_name().and_then(|n| n.to_str()) == Some("js2rust.lib") {
                return path.parent().map(|p| p.to_path_buf());
            }
        }
    }
    None
}

/// Map a Zig type string (from JSON) to the corresponding Rust C ABI type string.
fn zig_type_str_to_c_type(zig_type: &str) -> &'static str {
    match zig_type {
        "i64" | "i32" | "usize" => "i64",
        "f64" | "f32" => "f64",
        "bool" => "bool",
        "[]const u8" => "*const std::ffi::c_char",
        "void" => "()",
        _ => "i64",
    }
}

fn zig_ret_type_to_c_type(zig_type: &str) -> &'static str {
    match zig_type {
        "[]const u8" => "*mut std::ffi::c_char",
        _ => zig_type_str_to_c_type(zig_type),
    }
}

fn free_ptr_type(zig_type: &str) -> &'static str {
    match zig_type {
        "[]const u8" => "*mut std::ffi::c_char",
        _ => "*mut std::ffi::c_void",
    }
}

/// Generate Rust FFI extern "C" declarations from C ABI export JSON metadata.
fn generate_ffi_bindings(exports: &[serde_json::Value], path: &Path) {
    let mut code = String::new();
    code.push_str("// Auto-generated by js2rust-bridge/build.rs — Rust FFI bindings for js2rust Zig static library\n");
    code.push_str("// Do not edit manually.\n");
    code.push_str("// Linking is controlled by build.rs via cargo:rustc-link-lib=static=js2rust\n\n");
    code.push_str("unsafe extern \"C\" {\n");

    for exp in exports {
        let name = exp["name"].as_str().unwrap_or("");
        let params: &[serde_json::Value] = exp["params"]
            .as_array()
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let ret_type = exp["ret_type"].as_str().unwrap_or("i64");
        let has_free_func = exp["has_free_func"].as_bool().unwrap_or(false);

        let rust_params: Vec<String> = params
            .iter()
            .map(|p| {
                let p_name = p["name"].as_str().unwrap_or("arg");
                let p_type = p["zig_type"].as_str().unwrap_or("i64");
                let c_type = zig_type_str_to_c_type(p_type);
                format!("{}: {}", p_name, c_type)
            })
            .collect();

        let rust_ret = zig_ret_type_to_c_type(ret_type);

        code.push_str(&format!(
            "    pub fn {}({}) -> {};\n",
            sanitize_rust_name(name),
            rust_params.join(", "),
            rust_ret
        ));

        if has_free_func {
            code.push_str(&format!(
                "    pub fn free_{}(ptr: {});\n",
                sanitize_rust_name(name),
                free_ptr_type(ret_type)
            ));
        }
    }

    code.push_str("}\n");

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, &code).unwrap_or_else(|e| panic!("Failed to write FFI bindings: {}", e));
}

fn sanitize_rust_name(name: &str) -> String {
    name.to_string()
}
