// js2zig-core: public API + module declarations.

pub mod analyzer;
pub mod host;
pub mod parser;
pub mod project;
pub mod sourcemap;
pub mod testgen;
pub mod toml_config;
pub mod types;

/// Native-type system transpilation (anytype + @TypeOf).
pub mod native_proto;

// Pipeline module: transpile_project() orchestration.
pub mod pipeline;

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

/// Multi-file project configuration.
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    /// Project name (also used as Zig library name).
    pub name: String,
    /// JS source file paths. The first element is the entry point;
    /// additional elements are extra root files (multi-root mode).
    pub js_files: Vec<PathBuf>,
    /// Output directory path (typically $OUT_DIR).
    pub out_dir: PathBuf,
    /// Host function configuration (optional).
    pub host_config: Option<HostConfig>,
    /// Force rebuild (skip incremental cache).
    pub force_rebuild: bool,
    /// Whether to run `zig build` after transpilation.
    pub run_zig_build: bool,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "js2zig_lib".into(),
            js_files: vec![PathBuf::from("main.js")],
            out_dir: PathBuf::from("out"),
            host_config: None,
            force_rebuild: false,
            run_zig_build: false,
        }
    }
}

/// Result of transpiling a single group.
#[derive(Debug, Clone)]
pub struct GroupResult {
    /// Group name (e.g. "main", "utils").
    pub name: String,
    /// Whether this is a test group.
    pub is_test: bool,
    /// C ABI export metadata (serialized JSON).
    pub cabi_exports_json: String,
    /// Diagnostic messages.
    pub diagnostics: Vec<String>,
    /// Generated file paths.
    pub output_files: Vec<PathBuf>,
}

/// Result of transpiling a project (all groups).
#[derive(Debug, Default)]
pub struct ProjectResult {
    /// Per-group results.
    pub groups: Vec<GroupResult>,
    /// Global diagnostic messages.
    pub diagnostics: Vec<String>,
}

// ── Public API ───────────────────────────────────────────────────

/// Multi-file project transpilation: JS core file → Zig project + cabi_exports.json.
///
/// The core JS file and its transitive imports form a single group.
/// Does NOT run `zig build` (unless `config.run_zig_build == true`).
pub fn transpile_project(config: &ProjectConfig) -> Result<ProjectResult, String> {
    pipeline::transpile_project(config)
}

/// Write C ABI metadata for a single group project.
/// (Kept for backward compatibility; called from `pipeline.rs`.)
pub fn write_cabi_metadata(
    out_dir: &std::path::Path,
    group_name: &str,
    cabi_exports: &[(String, native_proto::NativeCabiExport)],
    host_fns: &host::HostFnRegistry,
    include_init: bool,
    cabi_rename: &std::collections::HashMap<String, String>,
) {
    pipeline::write_cabi_metadata(
        out_dir,
        group_name,
        cabi_exports,
        host_fns,
        include_init,
        cabi_rename,
    )
}
