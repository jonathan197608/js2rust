use crate::analyzer::{analyze_single_group, sanitize_module_name, strip_imports_extract_exports};
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

pub fn transpile_project(config: &ProjectConfig) -> Result<ProjectResult, String> {
    let ws = workspace_dir();

    // Derive input directory and core file name from js_file path.
    let js_file = &config.js_file;
    let in_path = js_file
        .parent()
        .ok_or_else(|| format!("cannot get parent directory of '{}'", js_file.display()))?
        .to_path_buf();
    let core_file = js_file
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("invalid file name in '{}'", js_file.display()))?
        .to_string();

    let in_dir = in_path.to_string_lossy().to_string();
    let out_dir: String = config.out_dir.to_string_lossy().to_string();
    let force_rebuild = config.force_rebuild;

    // Ensure output directory exists.
    fs::create_dir_all(&out_dir).map_err(|e| {
        format!("cannot create output directory '{}': {}", out_dir, e)
    })?;

    // === Phase 1: Analyze file groups (single core file + transitive deps) ===
    let (groups, groups_json) = analyze_single_group(&in_dir, &core_file);

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
        return Err(format!("no groups derived from core file '{}'", core_file));
    }

    // === Load host function registry ===
    let mut host_fns = crate::host::HostFnRegistry::new();

    // If host_config is provided in ProjectConfig, use it
    if let Some(ref host_config) = config.host_config {
        // Convert HostFunction to HostFnDef and register
        for f in &host_config.functions {
            let params: Vec<(String, crate::infer::ZigType)> = f.params.iter().enumerate().map(|(i, t)| {
                (format!("arg{}", i), crate::infer::ZigType::from(*t))
            }).collect();

            if f.is_async {
                if f.async_return_fields.is_empty() {
                    // Async with simple return type
                    let return_type = f.return_type
                        .map(crate::infer::ZigType::from)
                        .unwrap_or(crate::infer::ZigType::Void);
                    host_fns.register_async_simple(&f.name, &f.name, params, return_type);
                } else {
                    // Async with struct return type
                    let zig_name = f.struct_zig_name();
                    let c_name = f.struct_c_name();
                    let fields: Vec<crate::host::HostStructField> = f.async_return_fields.iter()
                        .map(|(name, ty)| {
                            crate::host::HostStructField {
                                name: name.clone(),
                                zig_type: ty.to_zig_field_type().to_string(),
                                c_type: ty.to_c_field_type().to_string(),
                            }
                        })
                        .collect();
                    let struct_def = crate::host::HostStructDef {
                        zig_name,
                        c_name,
                        fields,
                    };
                    host_fns.register_async(&f.name, &f.name, params, struct_def);
                }
            } else {
                let return_type = f.return_type.map(crate::infer::ZigType::from);
                host_fns.register(&f.name, params, return_type.unwrap_or(crate::infer::ZigType::Void));
            }
        }
    } else {
        // Fallback: load from host_config.json file
        let config_path = ws.join("host_config.json");
        if config_path.exists() {
            match crate::host::HostFnRegistry::load_from_file(&config_path) {
                Ok(registry) => {
                    host_fns = registry;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        } else {
            // No host_config.json found — this is normal for projects that don't use host functions.
        }
    };

    let mut builtins = crate::builtins::BuiltinRegistry::new();
    builtins.register_host_fns(&host_fns);
    let host_header = host_fns.generate_zig_header();
    let async_host_fn_names: Vec<String> = host_fns.async_fn_names();
    let runtime_dir = ws.join("runtime").to_string_lossy().to_string();

    // === Incremental compilation: load build cache ===
    let mut build_cache = read_build_cache(Path::new(&out_dir));
    let runtime_path = ws.join("runtime");
    let mut group_results: Vec<crate::GroupResult> = Vec::new();

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
            // Still collect the cabi_exports_json for the result
            let cabi_path = Path::new(&out_dir)
                .join(&group.core_name)
                .join("cabi_exports.json");
            let cabi_json = fs::read_to_string(&cabi_path).unwrap_or_default();
            group_results.push(crate::GroupResult {
                name: group.core_name.clone(),
                is_test: is_test_group,
                cabi_exports_json: cabi_json,
                diagnostics: Vec::new(),
                output_files: Vec::new(),
            });
            continue;
        }

        {
            let mut per_file_modules: Vec<crate::project::PerFileModule> = Vec::new();
            let mut all_module_exports: Vec<(String, String)> = Vec::new();
            let mut all_test_code = String::new();
            let mut combined_zig = String::new();
            let mut all_cabi_exports: Vec<crate::codegen::CabiExport> = Vec::new();
            let mut all_source_maps: Vec<crate::sourcemap::SourceMap> = Vec::new();
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
                // NOTE: In native_proto mode, exports are detected from JS source (export keyword).
                // This variable is passed to transpile_js() for accurate export detection.
                let codegen_exports: HashSet<String> = if is_test_group {
                    extract_all_function_names(stripped)
                } else if *member == group.core_file {
                    exports.clone()
                } else {
                    dep_re_exports.get(member).cloned().unwrap_or_default()
                };

                let allocator = oxc_allocator::Allocator::default();
                let program = crate::parser::parse(&allocator, stripped);

                // Collect host function return types and param types for type inference
                let mut host_return_types: std::collections::HashMap<String, crate::infer::ZigType> =
                    std::collections::HashMap::new();
                let mut host_param_types: std::collections::HashMap<String, Vec<crate::infer::ZigType>> =
                    std::collections::HashMap::new();
                for def in host_fns.iter() {
                    host_return_types.insert(def.name.clone(), def.ret_type.clone());
                    host_param_types.insert(
                        def.name.clone(),
                        def.params.iter().map(|(_, t)| t.clone()).collect(),
                    );
                }
                let host_struct_fields = host_fns.struct_fields_map();

                let async_fns: std::collections::HashSet<String> =
                    host_fns.async_fn_names().into_iter().collect();

                // Host function type info (currently not passed to native_proto)
                // TODO: Modify transpile_js() to accept host function info
                // for better type inference of host function calls.
                let _host_info = crate::codegen::HostTypeInfo {
                    return_types: &host_return_types,
                    param_types: &host_param_types,
                    struct_fields: &host_struct_fields,
                    async_fns: &async_fns,
                };

                // Use native_proto (strict static type system)
                let transpile_result = crate::native_proto::transpile_js(stripped, Some(codegen_exports));
                
                let (zig_code, diagnostics, closure_fns, fn_return_types, cabi_exports, source_map) =
                    match transpile_result {
                        Ok(result) => {
                            // Convert errors to diagnostics
                            let diagnostics: Vec<crate::infer::Diagnostic> = result.errors.iter()
                                .map(|err| crate::infer::Diagnostic {
                                    kind: crate::infer::DiagnosticKind::Error,
                                    span: None,
                                    message: err.clone(),
                                })
                                .collect();
                            
                            // Extract cabi_exports from result
                            let cabi_exports = result.cabi_exports;
                            
                            // closure_fns: not supported in native_proto yet, use empty
                            let closure_fns: std::collections::HashSet<String> = std::collections::HashSet::new();
                            
                            // fn_return_types: use var_types (convert to infer::ZigType)
                            let fn_return_types: std::collections::HashMap<String, crate::infer::ZigType> = 
                                result.var_types.iter()
                                    .map(|(k, v)| (k.clone(), v.to_infer_type()))
                                    .collect();
                            
                            // source_map: not generated by native_proto yet
                            let source_map = crate::sourcemap::SourceMap::new("");
                            
                            (result.zig_code, diagnostics, closure_fns, fn_return_types, cabi_exports, source_map)
                        }
                        Err(e) => {
                            // Return error as diagnostic
                            let diagnostics = vec![crate::infer::Diagnostic {
                                kind: crate::infer::DiagnosticKind::Error,
                                span: None,
                                message: e,
                            }];
                            (String::new(), diagnostics, HashSet::new(), HashMap::new(), Vec::<crate::codegen::CabiExport>::new(), crate::sourcemap::SourceMap::new(""))
                        }
                    };

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
                    // Test groups: also generate Zig test code
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
                host_header: if combined_zig.contains("host.") || !async_host_fn_names.is_empty() {
                    host_header.clone()
                } else {
                    String::new()
                },
                async_host_fn_names: async_host_fn_names.clone(),
                include_windows_stub: group_idx == 0,
            };

            match crate::project::generate(&project_opts) {
                Ok(()) => println!("  Generated: {}/{}", out_dir, group.core_name),
                Err(e) => {
                    eprintln!("  FAIL ({})", e);
                    continue;
                }
            }

            // Generate host.zig if host functions are registered
            if !host_fns.is_empty() {
                let host_zig_path = Path::new(&out_dir)
                    .join(&group.core_name)
                    .join("host.zig");
                let host_zig_content = host_fns.generate_zig_header();
                if let Err(e) = fs::write(&host_zig_path, &host_zig_content) {
                    eprintln!("  warning: failed to write host.zig: {}", e);
                } else {
                    println!("  Generated: {}/{}", out_dir, host_zig_path.file_name().unwrap().to_string_lossy());
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

            // Collect cabi_exports_json for the result
            let cabi_path = Path::new(&out_dir)
                .join(&group.core_name)
                .join("cabi_exports.json");
            let cabi_json = fs::read_to_string(&cabi_path).unwrap_or_default();

            group_results.push(crate::GroupResult {
                name: group.core_name.clone(),
                is_test: is_test_group,
                cabi_exports_json: cabi_json,
                diagnostics: Vec::new(),
                output_files: Vec::new(),
            });

            // Write test_cases.json for test groups (used by bridge test generation)
        }

        // === Zig build ===
        if !config.run_zig_build {
            // Skip zig build/test — caller handles compilation (e.g. proc-macro)
            continue;
        }

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

    // === Write build cache ===
    write_build_cache(Path::new(&out_dir), &build_cache);

    Ok(ProjectResult {
        groups: group_results,
        diagnostics: Vec::new(),
    })
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

        // Skip functions with JsValue/JsAny parameters (C ABI doesn't support unions)
        let has_js_obj_param = exp.params.iter().any(|(_, ty)| {
            *ty == crate::infer::ZigType::JsValue || *ty == crate::infer::ZigType::JsAny
        });
        if has_js_obj_param {
            // Re-export as const alias (no C ABI export)
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

        // ── Async exports: call _impl with js_runtime.getIo(), catch errors ──
        if exp.is_async {
            let async_zig_args = if zig_call_args.is_empty() {
                "js_runtime.getIo()".to_string()
            } else {
                format!("js_runtime.getIo(), {}", zig_call_args)
            };
            let async_cabi_args = if cabi_call_args.is_empty() {
                "js_runtime.getIo()".to_string()
            } else {
                format!("js_runtime.getIo(), {}", cabi_call_args)
            };

            if returns_string {
                // Zig-friendly adapter (for tests) — calls _impl directly
                out.push_str(&format!(
                    "pub fn {name}({params}) []const u8 {{\n    return {mod}.{name}({args}) catch @panic(\"async error\");\n}}\n",
                    name = name,
                    params = zig_params.join(", "),
                    mod = module,
                    args = async_zig_args,
                ));
                // C ABI wrapper (free_string scheme: result_len + dupeZ)
                let conversions = if cabi_to_zig_conversions.is_empty() {
                    String::new()
                } else {
                    format!("{}\n", cabi_to_zig_conversions.join("\n"))
                };
                let cabi_full_params = if cabi_params.is_empty() {
                    "result_len: *usize".to_string()
                } else {
                    format!("{}, result_len: *usize", cabi_params.join(", "))
                };
                out.push_str(&format!(
                    "pub export fn {name}_cabi({cabi_full_params}) [*:0]u8 {{\n{conv}    const _result = {mod}.{name}({args}) catch @panic(\"async error\");\n    const _result_cstr = allocator.dupeZ(u8, _result) catch unreachable;\n    result_len.* = _result.len;\n    return _result_cstr;\n}}\n",
                    name = name,
                    cabi_full_params = cabi_full_params,
                    conv = conversions,
                    mod = module,
                    args = async_cabi_args,
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
                let conversions = if cabi_to_zig_conversions.is_empty() {
                    String::new()
                } else {
                    format!("{}\n", cabi_to_zig_conversions.join("\n"))
                };

                if ret_zig == "void" {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) void {{\n{conv}    {mod}.{name}({args}) catch @panic(\"async error\");\n}}\n",
                        name = name,
                        params = cabi_params.join(", "),
                        conv = conversions,
                        mod = module,
                        args = async_cabi_args,
                    ));
                } else if exp_ret_is_js_value {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) i64 {{\n{conv}    const _result = {mod}.{name}({args}) catch @panic(\"async error\");\n    return _result.int;\n}}\n",
                        name = name,
                        params = cabi_params.join(", "),
                        conv = conversions,
                        mod = module,
                        args = async_cabi_args,
                    ));
                } else {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) {ret} {{\n{conv}    return {mod}.{name}({args}) catch @panic(\"async error\");\n}}\n",
                        name = name,
                        params = cabi_params.join(", "),
                        conv = conversions,
                        ret = ret_zig,
                        mod = module,
                        args = async_cabi_args,
                    ));
                }
            }

            out.push('\n');
            continue;
        }

        if returns_string {
            // ── Zig-friendly adapter (for tests) — calls _impl directly, no conversion ──
            out.push_str(&format!(
                "pub fn {name}({params}) []const u8 {{\n    return {mod}.{name}({args});\n}}\n",
                name = name,
                params = zig_params.join(", "),
                mod = module,
                args = zig_call_args,
            ));

            // ── C ABI wrapper (free_string scheme: result_len + dupeZ) ──
            let conversions = if cabi_to_zig_conversions.is_empty() {
                String::new()
            } else {
                format!("{}\n", cabi_to_zig_conversions.join("\n"))
            };
            let cabi_full_params = if cabi_params.is_empty() {
                "result_len: *usize".to_string()
            } else {
                format!("{}, result_len: *usize", cabi_params.join(", "))
            };
            out.push_str(&format!(
                "pub export fn {name}_cabi({cabi_full_params}) [*:0]u8 {{\n{conv}    const _result = {mod}.{name}({args});\n    const _result_cstr = allocator.dupeZ(u8, _result) catch unreachable;\n    result_len.* = _result.len;\n    return _result_cstr;\n}}\n",
                name = name,
                cabi_full_params = cabi_full_params,
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
            // Build params list
            let mut params: Vec<serde_json::Value> = exp
                .params
                .iter()
                .map(|(name, ty)| {
                    serde_json::json!({
                        "name": name,
                        "zig_type": ty.to_zig_str()
                    })
                })
                .collect();

            // For functions that need free_string: add result_len parameter
            // The Zig function signature is: fn name(params..., result_len: *usize) [*c]u8
            if exp.has_free_func {
                params.push(serde_json::json!({
                    "name": "result_len",
                    "zig_type": "*usize"
                }));
            }

            // Determine ret_type string for C ABI
            // If has_free_func: returns [*c]u8 (C pointer)
            // Otherwise: use to_cabi_str()
            let ret_type_str = if exp.has_free_func {
                "[*c]u8".to_string()
            } else {
                exp.ret_type.to_cabi_str()
            };

            serde_json::json!({
                "name": exp.name,
                "params": params,
                "ret_type": ret_type_str,
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
