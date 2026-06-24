//! js2rust.toml configuration parsing.
//!
//! Reads a single `js2rust.toml` from the crate root (`CARGO_MANIFEST_DIR`).
//! This file replaces the need to duplicate group/host-function information
//! between the `js2rust_bridge!()` macro and `build.rs`.
//!
//! ## Format
//!
//! ```toml
//! [project]
//! js_file = "js_src/main.js"           # required: core JS entry point
//! additional_js_files = ["js_src/extra.js"]  # optional
//!
//! [[host_functions]]
//! name = "host_add"
//! params = ["i64", "i64"]
//! returns = "i64"
//!
//! [[host_functions]]
//! name = "host_concat"
//! params = ["str", "str"]
//! returns = "str"
//!
//! [[host_functions]]
//! name = "fetch_user"
//! params = ["str"]
//! is_async = true
//! async_returns = { id = "i64", name = "str" }
//! ```

use indexmap::IndexMap;
use serde::Deserialize;
use std::path::PathBuf;

/// Root structure of `js2rust.toml`.
#[derive(Debug, Deserialize)]
pub struct Js2rustConfig {
    pub project: ProjectSection,
    #[serde(default)]
    pub host_functions: Vec<HostFnToml>,
}

/// `[project]` section.
#[derive(Debug, Deserialize)]
pub struct ProjectSection {
    /// Core JS source file path, relative to the crate root.
    pub js_file: String,
    /// Additional root JS files (multi-root mode), relative to the crate root.
    #[serde(default)]
    pub additional_js_files: Vec<String>,
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
                 js_file = \"js_src/main.js\"\n",
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

    /// Derive the Zig group name from the `js_file` file stem.
    ///
    /// The file stem is sanitized for Zig identifier rules.
    pub fn group_name(&self) -> String {
        let stem = std::path::Path::new(&self.project.js_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main");
        js2zig_core::analyzer::sanitize_module_name(stem)
    }
}
