use crate::analyzer::{analyze_groups, sanitize_module_name, strip_imports_extract_exports};
use crate::{ProjectConfig, ProjectResult};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve paths relative to the workspace root (parent of core crate).
fn workspace_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR must have a parent directory (workspace root)")
        .to_path_buf()
}

/// Build the dependency import list for a per-file Zig module.
/// Given a filename like "main.js", looks up its imported_names in the group
/// and maps source filenames to sanitized Zig module names.
fn build_dep_imports(
    filename: &str,
    group: &crate::analyzer::FileGroup,
) -> Vec<(String, String)> {
    let empty = Vec::new();
    let raw_imports = group.imported_names.get(filename).unwrap_or(&empty);
    raw_imports
        .iter()
        .map(|(imported_name, src_file)| {
            let mod_name = group
                .name_map
                .get(src_file.as_str())
                .cloned()
                .unwrap_or_else(|| {
                    let stem = src_file.strip_suffix(".js").unwrap_or(src_file);
                    sanitize_module_name(stem)
                });
            (imported_name.clone(), mod_name)
        })
        .collect()
}

/// Extract all top-level function names from JS source text.
/// Used for test groups where ALL functions should be CABI-exported (not just `export`-prefixed).
fn extract_all_function_names(source: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let rest = if let Some(r) = trimmed.strip_prefix("export function ") {
            r
        } else if let Some(r) = trimmed.strip_prefix("function ") {
            r
        } else {
            continue;
        };
        if let Some(paren) = rest.find('(') {
            let name = rest[..paren].trim();
            if !name.is_empty() {
                names.insert(name.to_string());
            }
        }
    }
    names
}

/// Scan zig_code for all `pub fn xxx(` and `pub export fn xxx(` declarations.
/// Returns the function names so the orchestrator can re-export them.
fn scan_pub_functions(zig_code: &str) -> Vec<String> {
    let mut fns = Vec::new();
    for line in zig_code.lines() {
        let trimmed = line.trim();
        let rest = if let Some(r) = trimmed.strip_prefix("pub export fn ") {
            r
        } else if let Some(r) = trimmed.strip_prefix("pub fn ") {
            r
        } else {
            continue;
        };
        if let Some(paren) = rest.find('(') {
            let name = rest[..paren].trim().to_string();
            // Skip infrastructure functions that shouldn't be re-exported
            if name != "init_js2rust" && name != "deinit_js2rust" {
                fns.push(name);
            }
        }
    }
    fns
}

/// Compute a content hash for a file group.
/// Hashes all member JS files + runtime .zig files so any change triggers rebuild.
fn compute_group_hash(
    in_dir: &Path,
    group: &crate::analyzer::FileGroup,
    runtime_dir: &Path,
) -> String {
    let mut hasher = std::hash::DefaultHasher::new();

    // Hash each member JS file content (sorted for determinism)
    let mut members: Vec<&String> = group.members.iter().collect();
    members.sort();
    for member in &members {
        member.hash(&mut hasher);
        if let Ok(content) = fs::read(in_dir.join(member)) {
            content.hash(&mut hasher);
        }
    }

    // Hash runtime .zig files (changes here affect all groups)
    if let Ok(entries) = fs::read_dir(runtime_dir) {
        let mut rt_files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "zig"))
            .collect();
        rt_files.sort();
        for rt_file in &rt_files {
            if let Ok(content) = fs::read(rt_file) {
                rt_file.file_name().hash(&mut hasher);
                content.hash(&mut hasher);
            }
        }
    }

    format!("{:016x}", hasher.finish())
}

/// Read build cache from `out/.build_cache.json`.
/// Returns group_name → hash_hex map.
fn read_build_cache(out_dir: &Path) -> HashMap<String, String> {
    let cache_path = out_dir.join(".build_cache.json");
    if let Ok(data) = fs::read_to_string(&cache_path)
        && let Ok(map) = serde_json::from_str(&data)
    {
        return map;
    }
    HashMap::new()
}

/// Write build cache to `out/.build_cache.json`.
fn write_build_cache(out_dir: &Path, cache: &HashMap<String, String>) {
    let cache_path = out_dir.join(".build_cache.json");
    if let Ok(json) = serde_json::to_string_pretty(cache) {
        let _ = fs::write(cache_path, json);
    }
}

/// A test case for bridge Rust test code generation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BridgeTestCase {
    /// Test name (e.g., "arr_len" from "test_arr_len")
    test_name: String,
    /// Function name being called (e.g., "testArrLen")
    fn_name: String,
    /// Rust-formatted arguments (e.g., "3.7" for f64 param)
    args: Vec<String>,
    /// Rust-formatted expected value (e.g., "5" for i64, "3.0" for f64)
    expected: Option<String>,
    /// CABI return type (e.g., "i64", "f64", "bool", "[]const u8")
    ret_type: String,
}

/// Convert Zig expected value to Rust format.
/// - `@as(i64, N)` → `N`
/// - `@as(f64, N)` → `N` (with `f64` suffix if needed)
/// - `true/false` → `true/false`
/// - `"string"` → `"string"`
fn zig_expected_to_rust(zig_expected: &str, ret_type: &str) -> String {
    let s = zig_expected.trim();

    // @as(i64, N) or @as(f64, N)
    if let Some(inner) = s.strip_prefix("@as(")
        && let Some(rest) = inner.strip_suffix(')')
        && let Some(comma) = rest.find(',')
    {
        let value = rest[comma + 1..].trim();
        return if ret_type == "f64" && !value.contains('.') {
            format!("{}.0", value)
        } else {
            value.to_string()
        };
    }

    // Bool
    if s == "true" || s == "false" {
        return s.to_string();
    }

    // String literal
    if s.starts_with('"') && s.ends_with('"') {
        return s.to_string();
    }

    // Null — not easily testable in Rust FFI
    if s == "null" {
        return s.to_string();
    }

    s.to_string()
}

/// Extract function name from an expression like "testArrLen()" or "cabiAdd(3, 5)".
fn extract_fn_name_from_expr(expr: &str) -> Option<String> {
    let paren = expr.find('(')?;
    let name = &expr[..paren];
    if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$') {
        Some(name.to_string())
    } else {
        None
    }
}

/// Extract argument strings from an expression like "cabiAdd(3, 5)".
fn extract_args_from_expr(expr: &str) -> Vec<String> {
    let Some(open) = expr.find('(') else {
        return vec![];
    };
    let Some(close) = expr.rfind(')') else {
        return vec![];
    };
    let inner = expr[open + 1..close].trim();
    if inner.is_empty() {
        return vec![];
    }
    // Simple comma split (doesn't handle nested calls, but good enough for test cases)
    inner.split(',').map(|a| a.trim().to_string()).collect()
}

/// Convert testgen test cases into bridge-compatible Rust test cases.
/// `skip_fns`: functions to exclude (e.g. JsAny returns that have no CABI export).
fn convert_test_cases_for_bridge(
    test_cases: &[crate::testgen::TestCase],
    cabi_ret_types: &HashMap<String, String>,
    skip_fns: &HashSet<String>,
) -> Vec<BridgeTestCase> {
    let mut result = Vec::new();
    for tc in test_cases {
        let Some(fn_name) = extract_fn_name_from_expr(&tc.expr_text) else {
            continue;
        };

        // Skip functions without CABI export (e.g. JsAny returns)
        if skip_fns.contains(&fn_name) {
            continue;
        }

        // Skip functions not in CABI exports (e.g. const arrow functions)
        let Some(ret_type) = cabi_ret_types.get(&fn_name).cloned() else {
            continue;
        };

        let test_name = tc.var_name
            .strip_prefix("test_")
            .unwrap_or(&tc.var_name)
            .to_string();

        let args = extract_args_from_expr(&tc.expr_text);

        let expected = tc.expected.as_ref().map(|e| zig_expected_to_rust(e, &ret_type));

        // Skip cases with null expected or complex expressions
        if expected.as_deref() == Some("null") {
            continue;
        }

        result.push(BridgeTestCase {
            test_name,
            fn_name,
            args,
            expected,
            ret_type,
        });
    }
    result
}

pub fn transpile_project(config: &ProjectConfig) -> Result<ProjectResult, String> {
    let ws = workspace_dir();
    let in_dir = config.js_dir.to_string_lossy().to_string();
    let out_dir: String = config.out_dir.to_string_lossy().to_string();
    let in_path = config.js_dir.clone();
    let force_rebuild = config.force_rebuild;

    // Ensure output directory exists.
    fs::create_dir_all(&out_dir).map_err(|e| {
        format!("cannot create output directory '{}': {}", out_dir, e)
    })?;

    // === Phase 1: Analyze file groups ===
    let (groups, groups_json) = analyze_groups(&in_dir);

    let groups_json_path = Path::new(&out_dir).join("groups.json");
    if let Err(e) = fs::write(&groups_json_path, &groups_json) {
        eprintln!(
            "warning: could not write '{}': {}",
            groups_json_path.display(),
            e
        );
    } else {
        println!("Wrote: {}/groups.json", out_dir);
    }

    if groups.is_empty() {
        return Err(format!("no core files found in '{}'", in_dir));
    }

    // === Load host function registry ===
    let config_path = ws.join("host_config.json");
    let host_fns = if config_path.exists() {
        match crate::host::HostFnRegistry::load_from_file(&config_path) {
            Ok(registry) => registry,
            Err(e) => {
                return Err(e);
            }
        }
    } else {
        eprintln!(
            "warning: '{}' not found — no host functions registered",
            config_path.display()
        );
        crate::host::HostFnRegistry::new()
    };

    let mut builtins = crate::builtins::BuiltinRegistry::new();
    builtins.register_host_fns(&host_fns);
    let host_header = host_fns.generate_zig_header();
    let async_wrappers = host_fns.generate_async_wrappers();
    let runtime_dir = ws.join("runtime").to_string_lossy().to_string();

    // === Incremental compilation: load build cache ===
    let mut build_cache = read_build_cache(Path::new(&out_dir));
    let runtime_path = ws.join("runtime");

    // === Phase 2: Generate Zig project per group ===
    // Always uses multi-file mode: one .zig per JS file + orchestrator lib.zig.
    for (group_idx, group) in groups.iter().enumerate() {
        let is_test_group = group.core_name.starts_with("test_");
        println!(
            "\n=== {} ({} member{}) {}===",
            group.core_name,
            group.members.len(),
            if group.members.len() == 1 { "" } else { "s" },
            if is_test_group { "[test] " } else { "" }
        );

        // --- Incremental check ---
        let current_hash = compute_group_hash(&in_path, group, &runtime_path);
        if !force_rebuild
            && let Some(cached_hash) = build_cache.get(&group.core_name)
            && *cached_hash == current_hash
        {
            println!("  unchanged, skipping (use --force to rebuild)");
            continue;
        }

        {
            let mut per_file_modules: Vec<crate::project::PerFileModule> = Vec::new();
            let mut all_module_exports: Vec<(String, String)> = Vec::new();
            let mut all_test_code = String::new();
            let mut combined_zig = String::new();
            let mut all_cabi_exports: Vec<crate::codegen::CabiExport> = Vec::new();
            let mut all_source_maps: Vec<crate::sourcemap::SourceMap> = Vec::new();
            let mut all_rust_test_cases: Vec<BridgeTestCase> = Vec::new();
            let mut has_error = false;

            // --- Pre-scan: collect source, exports, and module names ---
            struct MemberMeta {
                stripped: String,
                exports: HashSet<String>,
                module_name: String,
            }
            let mut member_metas: Vec<MemberMeta> = Vec::new();
            let mut core_exports: HashSet<String> = HashSet::new();

            for member in &group.members {
                let src = match fs::read_to_string(in_path.join(member)) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("  warning: cannot read '{}': {}", member, e);
                        continue;
                    }
                };
                let (stripped, exports) = strip_imports_extract_exports(&src);

                let module_name = group
                    .name_map
                    .get(member)
                    .cloned()
                    .unwrap_or_else(|| {
                        let stem = member.strip_suffix(".js").unwrap_or(member);
                        sanitize_module_name(stem)
                    });

                if *member == group.core_file {
                    core_exports = exports.clone();
                }

                member_metas.push(MemberMeta {
                    stripped,
                    exports,
                    module_name,
                });
            }

            // --- Compute re-exported names per dependency ---
            // Names that the core file re-exports from a dependency file should
            // also get `pub export fn` in that dependency module.
            let core_imports = group
                .imported_names
                .get(&group.core_file)
                .cloned()
                .unwrap_or_default();
            let mut dep_re_exports: HashMap<String, HashSet<String>> = HashMap::new();
            for (imported_name, source_file) in &core_imports {
                if core_exports.contains(imported_name) {
                    dep_re_exports
                        .entry(source_file.clone())
                        .or_default()
                        .insert(imported_name.clone());
                }
            }


            // --- Codegen pass ---
            for (member, meta) in group.members.iter().zip(member_metas.iter()) {
                let MemberMeta {
                    ref stripped,
                    ref exports,
                    ref module_name,
                } = *meta;

                if stripped.trim().is_empty() {
                    eprintln!("  skip '{}': empty after stripping imports", member);
                    continue;
                }

                // For test groups: export ALL functions for CABI bridge testing.
                // For normal groups: core file's JS exports → C ABI;
                //                    dependency file: only re-exported names → C ABI.
                let codegen_exports: HashSet<String> = if is_test_group {
                    extract_all_function_names(stripped)
                } else if *member == group.core_file {
                    exports.clone()
                } else {
                    dep_re_exports.get(member).cloned().unwrap_or_default()
                };

                let allocator = oxc_allocator::Allocator::default();
                let program = crate::parser::parse(&allocator, stripped);
                let (zig_code, diagnostics, closure_fns, fn_return_types, cabi_exports, source_map) =
                    crate::codegen::generate(&program, &builtins, &codegen_exports, stripped, member);

                let has_file_error = diagnostics
                    .iter()
                    .any(|d| d.kind == crate::infer::DiagnosticKind::Error);
                if has_file_error {
                    let err_count = diagnostics
                        .iter()
                        .filter(|d| d.kind == crate::infer::DiagnosticKind::Error)
                        .count();
                    eprintln!("  skip '{}': {} codegen error(s)", member, err_count);
                    for diag in &diagnostics {
                        if diag.kind == crate::infer::DiagnosticKind::Error {
                            eprintln!("    {}", diag.format_with_source(stripped));
                        }
                    }
                    has_error = true;
                    break;
                }

                if !diagnostics.is_empty() {
                    eprintln!("  '{}': {} diagnostic(s)", member, diagnostics.len());
                    for diag in &diagnostics {
                        eprintln!("    {}", diag.format_with_source(stripped));
                    }
                }

                for exp in exports {
                    all_module_exports.push((exp.clone(), module_name.clone()));
                }
                // Also scan all pub fn / pub export fn for test re-export
                for fn_name in scan_pub_functions(&zig_code) {
                    if !exports.contains(&fn_name) {
                        all_module_exports.push((fn_name, module_name.clone()));
                    }
                }

                let dep_imports = build_dep_imports(member, group);

                per_file_modules.push(crate::project::PerFileModule {
                    mod_name: module_name.clone(),
                    zig_code: zig_code.clone(),
                    dep_imports,
                });

                combined_zig.push_str(&zig_code);
                if !source_map.mappings.is_empty() {
                    all_source_maps.push(source_map);
                }

                // Always collect CABI exports for all groups
                all_cabi_exports.extend(cabi_exports);

                if is_test_group {
                    // Test groups: also generate Zig test code AND bridge test cases
                    let test_cases = crate::testgen::extract_test_cases(&program, stripped);
                    let closure_fn_refs: HashSet<&str> =
                        closure_fns.iter().map(|s| s.as_str()).collect();
                    let ret_type_map: HashMap<String, String> = fn_return_types
                        .iter()
                        .map(|(k, v)| (k.clone(), v.to_zig_str()))
                        .collect();
                    let file_test_code =
                        crate::testgen::generate_test_code(&test_cases, &closure_fn_refs, &ret_type_map);
                    all_test_code.push_str(&file_test_code);

                    // Convert to bridge Rust test cases
                    // Use all_cabi_exports (actual CABI-exported fns), NOT fn_return_types
                    // (which includes non-exportable functions like const arrow fns)
                    let cabi_ret_types: HashMap<String, String> = all_cabi_exports
                        .iter()
                        .map(|exp| (exp.name.clone(), exp.ret_type.to_cabi_str()))
                        .collect();
                    let skip_fns: HashSet<String> = fn_return_types
                        .iter()
                        .filter(|(_, v)| **v == crate::infer::ZigType::JsAny)
                        .map(|(k, _)| k.clone())
                        .collect();
                    let bridge_cases = convert_test_cases_for_bridge(&test_cases, &cabi_ret_types, &skip_fns);
                    all_rust_test_cases.extend(bridge_cases);
                }
            }

            if has_error {
                continue;
            }

            if per_file_modules.is_empty() {
                eprintln!("  skip: no valid modules after codegen");
                continue;
            }

            // --- Generate C ABI wrapper code for lib.zig ---
            let mut name_to_module: HashMap<&str, &str> = HashMap::new();
            for (exp_name, mod_name) in &all_module_exports {
                name_to_module.entry(exp_name).or_insert(mod_name);
            }
            let mut name_to_cabi: HashMap<&str, &crate::codegen::CabiExport> = HashMap::new();
            for exp in &all_cabi_exports {
                name_to_cabi.entry(&exp.name).or_insert(exp);
            }
            let cabi_wrapper_code = gen_cabi_wrappers(&name_to_module, &name_to_cabi);
            let cabi_names: HashSet<String> =
                name_to_cabi.keys().map(|&k| k.to_string()).collect();

            let project_opts = crate::project::ProjectOptions {
                name: group.core_name.clone(),
                out_dir: out_dir.clone(),
                per_file_code: per_file_modules,
                external_exports: all_module_exports,
                cabi_wrapper_code,
                cabi_names,
                test_code: all_test_code,
                runtime_dir: Some(runtime_dir.clone()),
                host_header: if combined_zig.contains("host.") {
                    host_header.clone()
                } else {
                    String::new()
                },
                async_host_wrappers: if combined_zig.contains("fetchUser") {
                    async_wrappers.clone()
                } else {
                    String::new()
                },
                include_windows_stub: group_idx == 0,
            };

            match crate::project::generate(&project_opts) {
                Ok(()) => println!("  Generated: {}/{}", out_dir, group.core_name),
                Err(e) => {
                    eprintln!("  FAIL ({})", e);
                    continue;
                }
            }

            // Write source map JSON
            if !all_source_maps.is_empty() {
                let sm_path = Path::new(&out_dir)
                    .join(&group.core_name)
                    .join("source_map.json");
                let sm_json = serde_json::json!({
                    "version": 1,
                    "generator": "js2rustc",
                    "files": all_source_maps
                        .iter()
                        .map(|sm| serde_json::json!({
                            "source": sm.source_file,
                            "mappings": sm.mappings.iter().map(|m| serde_json::json!({
                                "zig_line": m.zig_line,
                                "js_file": m.js_file,
                                "js_line": m.js_line,
                                "js_col": m.js_col,
                                "kind": m.kind,
                            })).collect::<Vec<_>>()
                        }))
                        .collect::<Vec<_>>()
                });
                if let Ok(json_str) = serde_json::to_string_pretty(&sm_json) {
                    let _ = std::fs::write(&sm_path, json_str);
                }
            }

            // Write CABI metadata for all groups
            // Only include init/deinit for the first non-test group
            let include_init = !is_test_group && group_idx == 0;
            write_cabi_metadata(Path::new(&out_dir), &group.core_name, &all_cabi_exports, &host_fns, include_init);

            // Write test_cases.json for test groups (used by bridge test generation)
            if is_test_group && !all_rust_test_cases.is_empty() {
                let tc_path = Path::new(&out_dir)
                    .join(&group.core_name)
                    .join("test_cases.json");
                if let Ok(json_str) = serde_json::to_string_pretty(&all_rust_test_cases) {
                    let _ = fs::write(&tc_path, json_str);
                }
            }
        }

        // === Zig build ===
        let project_path = Path::new(&out_dir).join(&group.core_name);
        let mut build_ok = false;
        let build_result = Command::new("zig")
            .arg("build")
            .current_dir(&project_path)
            .output();
        match build_result {
            Ok(result) if result.status.success() => {
                println!("  zig build: OK");
                build_ok = true;
            }
            Ok(result) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                eprintln!("  zig build FAILED:\n{}", stderr);
            }
            Err(_) => eprintln!("  warning: zig not found — skipping build"),
        }

        // === Zig tests ===
        let mut test_ok = false;
        let test_result = Command::new("zig")
            .arg("build")
            .arg("test")
            .current_dir(&project_path)
            .output();
        match test_result {
            Ok(result) if result.status.success() => {
                println!("  zig test: PASSED");
                test_ok = true;
            }
            Ok(result) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                eprintln!("  zig test FAILED:\n{}", stderr);
            }
            Err(_) => {}
        }

        // === Update build cache on success ===
        if build_ok && test_ok {
            build_cache.insert(group.core_name.clone(), current_hash.clone());
        }
    }

    // === Phase 3: Regenerate js2rust-bridge/src/lib.rs ===
    generate_bridge_lib_rs(&ws, Path::new(&out_dir));

    // === Write build cache ===
    write_build_cache(Path::new(&out_dir), &build_cache);

    // Return result (TODO: populate with actual group results)
    Ok(ProjectResult::default())
}

/// Regenerate `js2rust-bridge/src/lib.rs` based on current groups.
///
/// - Non-test groups → public `js2rust_bridge!()` invocations
/// - Test groups → `#[cfg(test)]` scoped `js2rust_bridge!()` + `#[test]` functions
pub fn generate_bridge_lib_rs(ws: &Path, out_dir: &Path) {
    let bridge_lib_path = ws.join("js2rust-bridge").join("src").join("lib.rs");
    if !bridge_lib_path.parent().unwrap().exists() {
        eprintln!("  warning: js2rust-bridge/src/ not found — skipping bridge generation");
        return;
    }

    // Collect normal groups and test groups
    let mut normal_groups: Vec<String> = Vec::new();
    let mut test_groups: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(out_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
                && path.join("cabi_exports.json").exists()
            {
                if name.starts_with("test_") {
                    test_groups.push(name.to_string());
                } else {
                    normal_groups.push(name.to_string());
                }
            }
        }
    }
    normal_groups.sort();
    test_groups.sort();

    if normal_groups.is_empty() && test_groups.is_empty() {
        eprintln!("  warning: no groups with cabi_exports.json — skipping bridge generation");
        return;
    }

    // Generate lib.rs content
    let mut out = String::new();
    out.push_str("// js2rust-bridge: Rust FFI bindings for translated JS/Zig code.\n");
    out.push_str("//\n");
    out.push_str("// AUTO-GENERATED by js2rustc — do not edit manually.\n");
    out.push_str("// Each `js2rust_bridge!()` invocation reads cabi_exports.json at compile time\n");
    out.push_str("// and generates `unsafe extern \"C\"` + safe Rust wrappers.\n\n");
    out.push_str("pub use js2rust_bridge_macro::js2rust_bridge;\n\n");
    out.push_str("pub mod host;\n\n");

    // === Normal groups: public FFI bindings ===
    out.push_str("// === Auto-generated FFI bindings for each group ===\n");
    for group in &normal_groups {
        out.push_str(&format!(
            "js2rust_bridge!(\"out/{}/cabi_exports.json\");\n",
            group
        ));
    }

    // === String conversion helpers ===
    out.push_str("\n// === String conversion helpers ===\n\n");
    out.push_str("\
/// Convert a null-terminated C string pointer to a Rust &str.
///
/// # Safety
/// The pointer must be a valid, null-terminated C string allocated by Zig.
/// The returned &str borrows the memory; call the corresponding `free_*`
/// function after use to release the memory.
pub unsafe fn cstr_to_str<'a>(ptr: *const std::ffi::c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
    c_str.to_str().ok()
}
");

    // === Test groups: #[cfg(test)] scoped bindings + #[test] functions ===
    if !test_groups.is_empty() {
        out.push_str("\n// === Test groups: FFI bindings + tests (only compiled during `cargo test`) ===\n\n");
        out.push_str("#[cfg(test)]\n");
        out.push_str("#[allow(non_snake_case)]\n");
        out.push_str("mod generated_tests {\n");
        out.push_str("    use js2rust_bridge_macro::js2rust_bridge;\n\n");

        // Import normal group wrappers for potential cross-group testing
        out.push_str("    // Re-import normal group wrappers\n");
        out.push_str("    #[allow(unused_imports)]\n");
        out.push_str("    use super::*;\n\n");

        // FFI bindings for test groups
        out.push_str("    // Test group FFI bindings\n");
        for group in &test_groups {
            out.push_str(&format!(
                "    js2rust_bridge!(\"out/{}/cabi_exports.json\");\n",
                group
            ));
        }

        // Generate #[test] functions from test_cases.json
        out.push_str("\n    // === Auto-generated test functions ===\n");

        // Track generated test names to detect duplicates across groups
        let mut used_test_names: HashSet<String> = HashSet::new();

        for group in &test_groups {
            let tc_path = Path::new(out_dir).join(group).join("test_cases.json");
            let test_cases: Vec<BridgeTestCase> = if let Ok(data) = fs::read_to_string(&tc_path) {
                serde_json::from_str(&data).unwrap_or_default()
            } else {
                continue;
            };

            if test_cases.is_empty() {
                continue;
            }

            // Short group suffix for disambiguation (strip "test_" prefix)
            let group_short = group.strip_prefix("test_").unwrap_or(group);

            out.push_str(&format!("\n    // --- {} ---\n", group));
            for tc in &test_cases {
                // Skip string-returning functions: bridge tests don't init Zig allocator,
                // and some string returns (e.g. trim) are sub-slices of constants that
                // crash on free. These are covered by Zig-side tests instead.
                if tc.ret_type == "[]const u8" {
                    continue;
                }

                let wrapper_name = format!("{}_{}", tc.fn_name, group);
                let args_str = tc.args.join(", ");
                let call = format!("{}({})", wrapper_name, args_str);

                // Disambiguate test names: if already used, append group suffix
                let base_name = tc.test_name.clone();
                let test_fn_name = if used_test_names.contains(&base_name) {
                    format!("{}_{}", base_name, group_short)
                } else {
                    base_name.clone()
                };
                used_test_names.insert(base_name);

                // Generate appropriate assertion based on return type
                if let Some(ref expected) = tc.expected {
                    if tc.ret_type == "f64" {
                        // Float comparison with tolerance
                        out.push_str(&format!(
                            "    #[test]\n    fn test_{name}() {{\n        assert!(({call} - {exp}f64).abs() < 1e-10, \"{name}: got {{}}, expected {exp}\", {call});\n    }}\n\n",
                            name = test_fn_name,
                            call = call,
                            exp = expected,
                        ));
                    } else if tc.ret_type == "bool" {
                        out.push_str(&format!(
                            "    #[test]\n    fn test_{name}() {{\n        assert_eq!({call}, {exp});\n    }}\n\n",
                            name = test_fn_name,
                            call = call,
                            exp = expected,
                        ));
                    } else {
                        // Default: integer comparison (i64)
                        out.push_str(&format!(
                            "    #[test]\n    fn test_{name}() {{\n        assert_eq!({call}, {exp});\n    }}\n\n",
                            name = test_fn_name,
                            call = call,
                            exp = expected,
                        ));
                    }
                } else {
                    // No expected value: smoke test (call should not panic)
                    out.push_str(&format!(
                        "    #[test]\n    fn test_{name}() {{\n        let _ = {call};\n    }}\n\n",
                        name = test_fn_name,
                        call = call,
                    ));
                }
            }
        }

        out.push_str("}\n");
    }

    // Write the file
    let total_groups = normal_groups.len() + test_groups.len();
    match fs::write(&bridge_lib_path, &out) {
        Ok(()) => println!(
            "\n  bridge: regenerated lib.rs ({} normal + {} test group{})",
            normal_groups.len(),
            test_groups.len(),
            if total_groups == 1 { "" } else { "s" }
        ),
        Err(e) => eprintln!("  error: cannot write bridge lib.rs: {}", e),
    }
}

/// Generate `pub export fn` wrapper code for lib.zig.
/// Each wrapper calls the per-file module function and lives in the root lib.zig,
/// so Zig correctly propagates the symbols into the final .lib.
///
/// For string-returning functions, ALSO generate a Zig-friendly adapter
/// (`pub fn greet(s: []const u8) []const u8`) so test code can call
/// the function with idiomatic Zig string types.
pub fn gen_cabi_wrappers(
    name_to_module: &HashMap<&str, &str>,
    name_to_cabi: &HashMap<&str, &crate::codegen::CabiExport>,
) -> String {
    use std::collections::HashSet;

    let mut out = String::new();
    let mut emitted: HashSet<&str> = HashSet::new();

    for (&name, exp) in name_to_cabi {
        if !emitted.insert(name) {
            continue;
        }
        let Some(&module) = name_to_module.get(name) else {
            continue;
        };

        let returns_string = exp.ret_type == crate::infer::ZigType::String;
        let ret_is_js_any = exp.ret_type == crate::infer::ZigType::JsAny;

        // JsAny returns: re-export as const alias (no CABI export).
        // This lets Zig test code call the function, but no C ABI symbol is emitted.
        if ret_is_js_any {
            out.push_str(&format!(
                "pub const {name} = {mod}.{name};\n\n",
                name = name,
                mod = module,
            ));
            continue;
        }

        // Build parameter lists for all function types
        let mut cabi_params: Vec<String> = Vec::new();
        let mut zig_params: Vec<String> = Vec::new();
        let mut arg_names: Vec<String> = Vec::new();
        let mut cabi_to_zig_conversions: Vec<String> = Vec::new();

        for (pname, ptype) in &exp.params {
            arg_names.push(pname.clone());
            if *ptype == crate::infer::ZigType::String {
                cabi_params.push(format!("{}: [*:0]const u8", pname));
                zig_params.push(format!("{}: []const u8", pname));
                cabi_to_zig_conversions.push(format!(
                    "    const {p}_slice: []const u8 = std.mem.span({p});",
                    p = pname
                ));
            } else {
                let zig_ty = ptype.to_zig_str();
                cabi_params.push(format!("{}: {}", pname, zig_ty));
                zig_params.push(format!("{}: {}", pname, zig_ty));
            }
        }

        // Build call args: for CABI wrapper, string params use _slice version
        let zig_call_args: String = arg_names.join(", ");
        let cabi_call_args: String = exp.params.iter().map(|(pname, ptype)| {
            if *ptype == crate::infer::ZigType::String {
                format!("{}_slice", pname)
            } else {
                pname.clone()
            }
        }).collect::<Vec<_>>().join(", ");

        if returns_string {
            // ── Zig-friendly adapter (for tests) — calls _impl directly, no conversion ──
            out.push_str(&format!(
                "pub fn {name}({params}) []const u8 {{\n    return {mod}.{name}_impl({args});\n}}\n",
                name = name,
                params = zig_params.join(", "),
                mod = module,
                args = zig_call_args,
            ));

            // ── C ABI wrapper — converts string params and return value ──
            let conversions = if cabi_to_zig_conversions.is_empty() {
                String::new()
            } else {
                format!("{}\n", cabi_to_zig_conversions.join("\n"))
            };
            out.push_str(&format!(
                "pub export fn {name}_cabi({params}) [*:0]const u8 {{\n{conv}    const _result = {mod}.{name}_impl({args});\n    return @ptrCast(_result.ptr);\n}}\n",
                name = name,
                params = cabi_params.join(", "),
                conv = conversions,
                mod = module,
                args = cabi_call_args,
            ));
            out.push_str(&format!(
                "comptime {{ @export(&{name}_cabi, .{{ .name = \"{name}\", .linkage = .strong }}); }}\n",
                name = name,
            ));
        } else {
            let ret_zig = if exp.ret_type == crate::infer::ZigType::Void {
                "void".to_string()
            } else {
                exp.ret_type.to_zig_str()
            };
            let exp_ret_is_js_value = exp.ret_type == crate::infer::ZigType::JsValue;

            if ret_zig == "void" {
                out.push_str(&format!(
                    "pub export fn {name}({params}) void {{\n    {mod}.{name}({args});\n}}\n",
                    name = name,
                    params = cabi_params.join(", "),
                    mod = module,
                    args = zig_call_args,
                ));
            } else if exp_ret_is_js_value {
                // JsValue: extract .int for C ABI (i64)
                out.push_str(&format!(
                    "pub export fn {name}({params}) i64 {{\n    const _result = {mod}.{name}({args});\n    return _result.int;\n}}\n",
                    name = name,
                    params = cabi_params.join(", "),
                    mod = module,
                    args = zig_call_args,
                ));
            } else {
                out.push_str(&format!(
                    "pub export fn {name}({params}) {ret} {{\n    return {mod}.{name}({args});\n}}\n",
                    name = name,
                    params = cabi_params.join(", "),
                    ret = ret_zig,
                    mod = module,
                    args = zig_call_args,
                ));
            }
        }

        // ── free_xxx wrapper (C ABI, always _cabi suffix) ──
        if exp.has_free_func {
            let free_fn = format!("free_{}", name);
            if returns_string {
                out.push_str(&format!(
                    "pub export fn {free_fn}_cabi(ptr: [*:0]const u8) void {{
    {mod}.free_{name}(ptr);
}}\n",
                    free_fn = free_fn,
                    name = name,
                    mod = module,
                ));
                out.push_str(&format!(
                "comptime {{ @export(&{free_fn}_cabi, .{{ .name = \"{free_fn}\", .linkage = .strong }}); }}\n",
                    free_fn = free_fn,
                ));
            } else {
                // Closure return: free takes *anyopaque
                out.push_str(&format!(
                    "pub export fn {free_fn}(ptr: *anyopaque) void {{
    {mod}.{free_fn}(ptr);
}}\n",
                    free_fn = free_fn,
                    mod = module,
                ));
            }
        }

        out.push('\n');
    }

    out
}

/// Write C ABI exports/imports JSON metadata for a single group project.
pub fn write_cabi_metadata(
    out_dir: &Path,
    group_name: &str,
    cabi_exports: &[crate::codegen::CabiExport],
    host_fns: &crate::host::HostFnRegistry,
    include_init: bool,
) {
    let project_dir = out_dir.join(group_name);

    // cabi_exports.json — filter out JsAny returns (no C ABI export generated)
    let exports_path = project_dir.join("cabi_exports.json");
    let mut exports_value: Vec<serde_json::Value> = cabi_exports
        .iter()
        .filter(|exp| exp.ret_type != crate::infer::ZigType::JsAny)
        .map(|exp| {
            let params: Vec<serde_json::Value> = exp
                .params
                .iter()
                .map(|(name, ty)| {
                    serde_json::json!({
                        "name": name,
                        "zig_type": ty.to_zig_str()
                    })
                })
                .collect();
            serde_json::json!({
                "name": exp.name,
                "params": params,
                "ret_type": exp.ret_type.to_cabi_str(),
                "has_free_func": exp.has_free_func
            })
        })
        .collect();

    // Only include js2rust_init and js2rust_deinit for the first non-test group
    if include_init {
        exports_value.push(serde_json::json!({
            "name": "js2rust_init",
            "params": [],
            "ret_type": "void",
            "has_free_func": false
        }));
        exports_value.push(serde_json::json!({
            "name": "js2rust_deinit",
            "params": [],
            "ret_type": "void",
            "has_free_func": false
        }));
    }

    if let Ok(json_str) = serde_json::to_string_pretty(&exports_value) {
        let _ = fs::write(&exports_path, &json_str);
    }

    // cabi_imports.json
    let imports_path = project_dir.join("cabi_imports.json");
    let imports_value = host_fns.to_json_value();
    if let Ok(json_str) = serde_json::to_string_pretty(&imports_value) {
        let _ = fs::write(&imports_path, &json_str);
    }
}
