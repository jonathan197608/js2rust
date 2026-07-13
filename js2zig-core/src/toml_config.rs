//! js2rust.toml configuration types and loading logic.
//!
//! Single source of truth for TOML configuration, shared by:
//! - `js2rust-bridge` (build.rs path)
//! - `js2rust-bridge-macro` (proc-macro path)
//!
//! Both crates previously duplicated these types and the loading function.
//! Now they both delegate here.

use crate::analyzer::sanitize_module_name;
use crate::{HostConfig, HostFunction, HostType};
use indexmap::IndexMap;
use serde::Deserialize;
use std::path::PathBuf;

/// Root structure of `js2rust.toml`.
#[derive(Debug, Deserialize)]
pub struct Js2rustConfig {
    /// `[project]` section.
    pub project: ProjectSection,
    /// `[build]` section — controls build.rs behavior.
    #[serde(default)]
    pub build: BuildSection,
    /// `[[host_functions]]` entries.
    #[serde(default)]
    pub host_functions: Vec<HostFnToml>,
}

/// `[project]` section.
#[derive(Debug, Deserialize)]
pub struct ProjectSection {
    /// JS source file paths (relative to crate root).
    /// The first element is the entry point; additional elements are extra roots.
    pub js_files: Vec<String>,
}

/// `[build]` section — controls build.rs behavior.
#[derive(Debug, Default, Deserialize)]
pub struct BuildSection {
    /// Force rebuild (skip incremental cache). Default: false.
    #[serde(default)]
    pub force_rebuild: bool,
    /// Whether to run `zig build` after transpilation. Default: false.
    #[serde(default)]
    pub run_zig_build: bool,
    /// Zig optimization level passed as `-Doptimize=...` to `zig build`.
    ///
    /// Valid values: `"Debug"`, `"ReleaseSafe"`, `"ReleaseFast"`, `"ReleaseSmall"`.
    /// When set, this overrides the automatic inference from the Cargo profile.
    /// Default: `None` (auto-detect from Cargo profile).
    #[serde(default)]
    pub zig_optimize: Option<String>,
    /// Enable ICU4X-based implementations for ICU-dependent string methods
    /// (localeCompare, normalize, toLocaleUpperCase, toLocaleLowerCase).
    /// When `true`, js_string_icu.zig is overwritten with the ICU4X version
    /// that delegates to host_icu_* C ABI functions provided by js2rust-bridge.
    /// When `false` (default), the simplified runtime version is used, which
    /// provides locale-unaware fallbacks suitable for transpiler testing.
    #[serde(default)]
    pub icu: bool,
}

/// A single `[[host_functions]]` entry.
#[derive(Debug, Deserialize)]
pub struct HostFnToml {
    /// Host function name (must match `host_` prefix convention in JS).
    pub name: String,
    /// Parameter types: "i64", "i32", "f64", "bool", "str".
    pub params: Vec<String>,
    /// Return type (optional for async struct returns).
    #[serde(default)]
    pub returns: Option<String>,
    /// Whether this is an async host function.
    #[serde(default)]
    pub is_async: bool,
    /// For async functions that return a struct: field_name → field_type.
    #[serde(default)]
    pub async_returns: IndexMap<String, String>,
}

impl Js2rustConfig {
    /// Load `js2rust.toml` from the crate manifest directory.
    ///
    /// Panics with a descriptive message if the file is missing or malformed.
    pub fn from_manifest_dir() -> Self {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let config_path = PathBuf::from(&manifest_dir).join("js2rust.toml");

        let content = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
            panic!(
                "js2rust_bridge: failed to read {}: {}\n\
                 Create a js2rust.toml in your crate root with:\n\
                 \n\
                 [project]\n\
                 js_files = [\"js_src/main.js\"]\n",
                config_path.display(),
                e
            );
        });

        toml::from_str(&content).unwrap_or_else(|e| {
            panic!(
                "js2rust_bridge: failed to parse {}: {}\n\
                 Make sure your js2rust.toml is valid TOML.",
                config_path.display(),
                e
            );
        })
    }

    /// Derive the Zig project name from the first `js_files` entry's file stem.
    pub fn project_name(&self) -> String {
        let default = "main.js".to_string();
        let first = self.project.js_files.first().unwrap_or(&default);
        let stem = std::path::Path::new(first)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main");
        sanitize_module_name(stem)
    }

    /// Convert `host_functions` TOML entries to `HostConfig`.
    ///
    /// Returns `None` if there are no host functions.
    pub fn to_host_config(&self) -> Option<HostConfig> {
        if self.host_functions.is_empty() {
            return None;
        }
        let functions: Vec<HostFunction> = self
            .host_functions
            .iter()
            .map(|hf| {
                let params: Vec<HostType> = hf
                    .params
                    .iter()
                    .map(|t| HostType::from_toml_str(t))
                    .collect();

                let return_type = hf.returns.as_deref().and_then(|t| {
                    if t == "void" {
                        None
                    } else {
                        Some(HostType::from_toml_str(t))
                    }
                });

                let async_return_fields: Vec<(String, HostType)> = hf
                    .async_returns
                    .iter()
                    .map(|(name, ty)| (name.clone(), HostType::from_toml_str(ty)))
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

        Some(HostConfig { functions })
    }
}

/// Convert snake_case to PascalCase.
///
/// Shared utility used by both bridge and bridge-macro for generating
/// struct names from host function names (e.g. "fetch_user" → "FetchUser").
pub fn pascal_case(name: &str) -> String {
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

impl HostType {
    /// Convert a TOML type name string to `HostType`.
    ///
    /// Used by both bridge and bridge-macro to parse `js2rust.toml` host function types.
    /// This was previously duplicated as `type_name_to_host_type()` in both crates.
    pub fn from_toml_str(name: &str) -> HostType {
        match name {
            "i64" => HostType::I64,
            "i32" => HostType::I32,
            "f64" => HostType::F64,
            "bool" => HostType::Bool,
            "str" => HostType::Str,
            "void" => HostType::Void,
            other => panic!(
                "js2rust.toml: unknown host type '{}'. \
                 Valid types: i64, i32, f64, bool, str, void",
                other
            ),
        }
    }
}
