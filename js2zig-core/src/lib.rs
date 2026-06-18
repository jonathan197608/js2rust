// js2zig-core: public API + module declarations.

pub mod analyzer;
pub mod builtins;
pub mod codegen;
pub mod host;
pub mod infer;
pub mod parser;
pub mod project;
pub mod sourcemap;
pub mod testgen;

// Pipeline module: transpile_project() orchestration.
pub mod pipeline;

// ── Public API types ─────────────────────────────────────────────

use std::path::PathBuf;

/// Multi-file project configuration.
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    /// Project name (also used as Zig library name).
    pub name: String,
    /// JS source file directory path.
    pub js_dir: PathBuf,
    /// Output directory path (typically $OUT_DIR).
    pub out_dir: PathBuf,
    /// Host function configuration file path (optional).
    pub host_config: Option<PathBuf>,
    /// Force rebuild (skip incremental cache).
    pub force_rebuild: bool,
    /// Whether to run `zig build` after codegen.
    pub run_zig_build: bool,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "js2zig_lib".into(),
            js_dir: PathBuf::from("in"),
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

/// Multi-file project transpilation: JS directory → Zig projects + cabi_exports.json.
///
/// This is equivalent to `main.rs` in js2rustc, but exposed as a library function.
/// It does NOT run `zig build` (unless `config.run_zig_build == true`),
/// and does NOT generate `js2rust-bridge/src/lib.rs`
/// (that is the responsibility of `js2zig-build`).
pub fn transpile_project(config: &ProjectConfig) -> Result<ProjectResult, String> {
    pipeline::transpile_project(config)
}

/// Write C ABI metadata for a single group project.
/// (Kept for backward compatibility; called from `pipeline.rs`.)
pub fn write_cabi_metadata(
    out_dir: &std::path::Path,
    group_name: &str,
    cabi_exports: &[codegen::CabiExport],
    host_fns: &host::HostFnRegistry,
    include_init: bool,
) {
    pipeline::write_cabi_metadata(out_dir, group_name, cabi_exports, host_fns, include_init)
}
