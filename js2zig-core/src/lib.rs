// js2zig-core: public API + module declarations.

pub mod analyzer;
pub mod host;
pub mod parser;
pub mod project;
pub mod testgen;
pub mod toml_config;
pub mod types;

/// Native-type system transpilation (anytype + @TypeOf).
pub mod native_proto;

// Pipeline module: transpile_project() orchestration.
pub mod pipeline;

// C ABI wrapper generation and metadata.
pub mod cabi;

/// ZigIR — structured intermediate representation between AST and Zig source.
pub mod zigir;

// ── Submodules ──
pub(crate) mod infer;
pub(crate) mod jsdoc;
pub(crate) mod native_builtins;
#[cfg(test)]
pub(crate) mod tests;

use std::path::PathBuf;

/// Host function type for FFI binding generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostType {
    Void,
    Bool,
    I32,
    I64,
    F64,
    Str, // *const c_char
}

/// Host function description for FFI binding generation.
#[derive(Debug, Clone)]
pub struct HostFunction {
    pub name: String,
    pub params: Vec<HostType>,
    pub return_type: Option<HostType>,
    /// Whether this is an async function (called with `await` from JS).
    pub is_async: bool,
    /// For async functions returning a struct: field name and type pairs.
    /// Empty for sync functions or async functions with simple return types.
    pub async_return_fields: Vec<(String, HostType)>,
}

impl HostFunction {
    /// Derive a PascalCase struct name from the function name.
    /// e.g. "fetch_user" → "FetchUserResult"
    pub fn struct_zig_name(&self) -> String {
        format!("{}Result", toml_config::pascal_case(&self.name))
    }

    /// Derive the C ABI extern struct name.
    /// e.g. "fetch_user" → "HostFetchUserResult"
    pub fn struct_c_name(&self) -> String {
        format!("Host{}", self.struct_zig_name())
    }
}

impl HostType {
    /// Convert to the clean Zig type string used in wrapper structs.
    pub fn to_zig_field_type(self) -> &'static str {
        match self {
            HostType::Void => "void",
            HostType::Bool => "bool",
            HostType::I32 => "i32",
            HostType::I64 => "i64",
            HostType::F64 => "f64",
            HostType::Str => "[]const u8",
        }
    }

    /// Convert to the C ABI type string used in extern structs.
    pub fn to_c_field_type(self) -> &'static str {
        match self {
            HostType::Void => "void",
            HostType::Bool => "bool",
            HostType::I32 => "i32",
            HostType::I64 => "i64",
            HostType::F64 => "f64",
            HostType::Str => "[256]u8",
        }
    }
}

/// Host function configuration.
#[derive(Debug, Clone)]
pub struct HostConfig {
    pub functions: Vec<HostFunction>,
}

/// Build behavior configuration.
#[derive(Debug, Clone, Default)]
pub struct BuildConfig {
    /// Force rebuild (skip incremental cache).
    pub force_rebuild: bool,
    /// Whether to run `zig build` after transpilation.
    pub run_zig_build: bool,
    /// Zig optimization level passed as `-Doptimize=...` to `zig build`.
    ///
    /// Valid values: `"Debug"`, `"ReleaseSafe"`, `"ReleaseFast"`, `"ReleaseSmall"`.
    /// If `None`, the caller should infer from the Cargo profile automatically.
    pub zig_optimize: Option<String>,
    /// When true, the caller is a Cargo build script (`build.rs`).
    /// This controls progress output only (project headers, cache status,
    /// "Generated:" paths, "zig build: OK", etc.) which Cargo filters
    /// and prefixes with `[package]`.
    ///
    /// Diagnostic output (errors, warnings, @compileError messages) is
    /// always shown regardless of this flag, so users see actionable
    /// information even in proc-macro context.
    ///
    /// When false (proc-macro context), only diagnostics are emitted;
    /// progress noise is suppressed because proc-macro stdout leaks
    /// directly to the terminal unfiltered.
    pub is_build_script: bool,
}

/// Multi-file project configuration.
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    /// Absolute path to the directory containing JS source files (e.g. `.../js_src/`).
    pub js_dir: PathBuf,
    /// JS entry file names (relative to `js_dir`).
    /// The first entry is the primary root; additional entries are extra roots
    /// whose exports become C ABI-exportable.
    pub js_files: Vec<String>,
    /// Output directory path (typically $OUT_DIR).
    pub out_dir: PathBuf,
    /// Host function configuration (optional).
    pub host_config: Option<HostConfig>,
    /// Build behavior configuration.
    pub build: BuildConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            js_dir: PathBuf::from("."),
            js_files: vec!["main.js".to_string()],
            out_dir: PathBuf::from("out"),
            host_config: None,
            build: BuildConfig::default(),
        }
    }
}

/// Result of transpiling a project.
#[derive(Debug, Default)]
pub struct ProjectResult {
    /// Sanitized project name (derived from entry file stem).
    pub project_name: String,
    /// Whether this is a test project (entry file starts with "test_").
    pub is_test: bool,
    /// C ABI export metadata (serialized JSON).
    pub cabi_exports_json: String,
    /// Diagnostic messages.
    pub diagnostics: Vec<String>,
}

// ── Public API ───────────────────────────────────────────────────

/// Multi-file project transpilation: JS entry file → Zig project + cabi_exports.json.
///
/// The entry JS file and its transitive imports form a single project.
/// Does NOT run `zig build` (unless `config.build.run_zig_build == true`).
///
/// `js_dir` is the absolute path to the directory containing JS source files.
/// `js_files` are filenames relative to `js_dir`; the first entry is the
/// primary entry point whose stem becomes the project name.
pub fn transpile_project(config: &ProjectConfig) -> Result<ProjectResult, String> {
    pipeline::transpile_project(config)
}

pub use cabi::write_cabi_metadata;
