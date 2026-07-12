use crate::analyzer::{analyze_single_group, sanitize_module_name};
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
fn build_dep_imports(filename: &str, group: &crate::analyzer::FileGroup) -> Vec<(String, String)> {
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

/// Save per-group diagnostics to `out/<group>/diagnostics.json`.
/// On cache hit, these are loaded back so the caller always sees
/// consistent diagnostic output without re-transpiling.
fn save_diagnostics(out_dir: &Path, group_name: &str, diagnostics: &[String]) {
    let path = Path::new(out_dir).join(group_name).join("diagnostics.json");
    if let Ok(json) = serde_json::to_string_pretty(diagnostics) {
        let _ = fs::write(path, json);
    }
}

pub fn transpile_project(config: &ProjectConfig) -> Result<ProjectResult, String> {
    let ws = workspace_dir();

    // Derive input directory and core file name from the entry point.
    let js_file = &config.entry_file;
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
    let verbose = config.is_build_script; // only print progress in build.rs context

    // Ensure output directory exists.
    fs::create_dir_all(&out_dir)
        .map_err(|e| format!("cannot create output directory '{}': {}", out_dir, e))?;

    // === Phase 1: Analyze file groups (single or multi-root core files + transitive deps) ===
    let additional_js_files: Vec<String> = config
        .additional_roots
        .iter()
        .filter_map(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .collect();
    let (group, groups_json) = analyze_single_group(&in_dir, &core_file, &additional_js_files);

    // Emit cargo:rerun-if-changed for every JS file discovered by the analyzer
    // (including transitive dependencies not listed in js2rust.toml).
    // These directives take effect in subsequent builds — Cargo stores them
    // and uses them to decide whether to re-run the build script.
    // Only emit when called from build.rs (is_build_script=true);
    // proc-macros cannot use these directives and their stdout would leak
    // noise to the terminal.
    if config.is_build_script {
        for member in &group.members {
            let member_path = Path::new(&in_dir).join(member);
            println!("cargo:rerun-if-changed={}", member_path.display());
        }
    }

    let groups_json_path = Path::new(&out_dir).join("groups.json");
    if let Err(e) = fs::write(&groups_json_path, &groups_json) {
        if verbose {
            eprintln!(
                "warning: could not write '{}': {}",
                groups_json_path.display(),
                e
            );
        }
    } else if verbose {
        println!("Wrote: {}/groups.json", out_dir);
    }

    if group.members.is_empty() {
        return Err(format!("no members derived from core file '{}'", core_file));
    }

    // === Load host function registry ===
    let mut host_fns = crate::host::HostFnRegistry::new();

    // If host_config is provided in ProjectConfig, use it
    if let Some(ref host_config) = config.host_config {
        // Convert HostFunction to HostFnDef and register
        for f in &host_config.functions {
            let params: Vec<(String, crate::types::ZigType)> = f
                .params
                .iter()
                .enumerate()
                .map(|(i, t)| (format!("arg{}", i), crate::types::ZigType::from(*t)))
                .collect();

            if f.is_async {
                if f.async_return_fields.is_empty() {
                    // Async with simple return type
                    let return_type = f
                        .return_type
                        .map(crate::types::ZigType::from)
                        .unwrap_or(crate::types::ZigType::Void);
                    host_fns.register_async_simple(&f.name, &f.name, params, return_type);
                } else {
                    // Async with struct return type
                    let zig_name = f.struct_zig_name();
                    let c_name = f.struct_c_name();
                    let fields: Vec<crate::host::HostStructField> = f
                        .async_return_fields
                        .iter()
                        .map(|(name, ty)| crate::host::HostStructField {
                            name: name.clone(),
                            zig_type: ty.to_zig_field_type().to_string(),
                            c_type: ty.to_c_field_type().to_string(),
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
                let return_type = f.return_type.map(crate::types::ZigType::from);
                host_fns.register(
                    &f.name,
                    params,
                    return_type.unwrap_or(crate::types::ZigType::Void),
                );
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

    let host_header = host_fns.generate_zig_header();
    let async_host_fn_names: Vec<String> = host_fns.async_fn_names();
    let runtime_dir = ws.join("runtime").to_string_lossy().to_string();

    // === Incremental compilation: load build cache ===
    let mut build_cache = read_build_cache(Path::new(&out_dir));
    let runtime_path = ws.join("runtime");
    let mut group_results: Vec<crate::GroupResult> = Vec::new();

    // === Phase 2: Generate Zig project ===
    // Always uses multi-file mode: one .zig per JS file + orchestrator lib.zig.
    'group_block: {
        let group_idx = 0;
        let is_test_group = group.core_name.starts_with("test_");
        if verbose {
            println!(
                "\n=== {} ({} member{}) {}===",
                group.core_name,
                group.members.len(),
                if group.members.len() == 1 { "" } else { "s" },
                if is_test_group { "[test] " } else { "" }
            );
        }

        // --- Incremental check ---
        let current_hash = compute_group_hash(&in_path, &group, &runtime_path);
        if !force_rebuild
            && let Some(cached_hash) = build_cache.get(&group.core_name)
            && *cached_hash == current_hash
        {
            if verbose {
                println!("  unchanged (cache hit)");
            }
            // Still collect the cabi_exports_json for the result
            let cabi_path = Path::new(&out_dir)
                .join(&group.core_name)
                .join("cabi_exports.json");
            let cabi_json = fs::read_to_string(&cabi_path).unwrap_or_default();

            // Load cached diagnostics so the caller always sees consistent output.
            // Re-transpilation saves to this file; cache hit loads from it.
            let diag_path = Path::new(&out_dir)
                .join(&group.core_name)
                .join("diagnostics.json");
            let cached_diagnostics: Vec<String> = fs::read_to_string(&diag_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            group_results.push(crate::GroupResult {
                name: group.core_name.clone(),
                is_test: is_test_group,
                cabi_exports_json: cabi_json,
                diagnostics: cached_diagnostics,
            });
        } else {
            // Hash mismatch (or force_rebuild) — re-transpile this group.
            if force_rebuild {
                if verbose {
                    println!("  force rebuild");
                }
            } else if verbose {
                println!("  source changed, re-transpiling");
            }

            {
                let mut per_file_modules: Vec<crate::project::PerFileModule> = Vec::new();
                let mut all_module_exports: Vec<(String, String)> = Vec::new();
                let mut all_test_code = String::new();
                let combined_zig = String::new();
                let mut all_cabi_exports: Vec<(String, crate::types::NativeCabiExport)> =
                    Vec::new();
                let all_source_maps: Vec<crate::sourcemap::SourceMap> = Vec::new();
                let has_error = false;
                let mut file_diagnostics: Vec<String> = Vec::new();

                // --- Transpile pass (all metadata from group AST, no source scanning) ---
                let core_exports = group
                    .exported_names
                    .get(&group.core_file)
                    .cloned()
                    .unwrap_or_default();

                // --- Compute re-exported names per dependency ---
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

                // Additional core files (multi-root): treat their exports as CABI-exportable too.
                let additional_core_set: HashSet<String> =
                    additional_js_files.iter().cloned().collect();

                for member in &group.members {
                    let src = match group.file_sources.get(member) {
                        Some(s) => s.clone(),
                        None => {
                            eprintln!("  skip '{}': no cached source", member);
                            continue;
                        }
                    };

                    if src.trim().is_empty() {
                        eprintln!("  skip '{}': empty source", member);
                        continue;
                    }

                    let module_name = group.name_map.get(member).cloned().unwrap_or_else(|| {
                        let stem = member.strip_suffix(".js").unwrap_or(member);
                        sanitize_module_name(stem)
                    });

                    // For test groups: ALL toplevel functions → C ABI.
                    // For normal groups: core file's JS exports → C ABI;
                    //                    additional core files → C ABI (full exports);
                    //                    dependency file: only re-exported names → C ABI.
                    let transpile_exports: HashSet<String> = if is_test_group {
                        group.all_fn_names.get(member).cloned().unwrap_or_default()
                    } else if *member == group.core_file || additional_core_set.contains(member) {
                        group
                            .exported_names
                            .get(member)
                            .cloned()
                            .unwrap_or_default()
                    } else {
                        dep_re_exports.get(member).cloned().unwrap_or_default()
                    };

                    let exports_for_all_modules = transpile_exports.clone();

                    let program = match group.parsed_programs.get(member) {
                        Some(p) => p,
                        None => {
                            eprintln!("  skip '{}': no parsed program in group", member);
                            continue;
                        }
                    };

                    let transpile_result = crate::native_proto::transpile_js(
                        program,
                        &src,
                        Some(transpile_exports),
                        Some(&host_fns),
                        member,
                    );

                    let (zig_code, cabi_exports, has_error) = match transpile_result {
                        Ok(result) => {
                            // @compileError diagnostics (non-blocking)
                            if !result.compile_errors.is_empty() {
                                eprintln!(
                                    "  '{}': {} unsupported feature(s):",
                                    member,
                                    result.compile_errors.len()
                                );
                                for msg in &result.compile_errors {
                                    eprintln!("    @compileError: {}", msg);
                                }
                                for msg in &result.compile_errors {
                                    file_diagnostics
                                        .push(format!("{}: COMPILE_ERROR - {}", member, msg));
                                }
                            }

                            // Hard errors — block file generation
                            let hard_errors: Vec<&str> = result
                                .errors
                                .iter()
                                .filter(|m| !m.contains("(Rule 8)"))
                                .map(|s| s.as_str())
                                .collect();
                            // Rule 8 soft errors — non-blocking warnings
                            let rule8_errors: Vec<&str> = result
                                .errors
                                .iter()
                                .filter(|m| m.contains("(Rule 8)"))
                                .map(|s| s.as_str())
                                .collect();

                            if !hard_errors.is_empty() {
                                eprintln!(
                                    "  skip '{}': {} transpile error(s)",
                                    member,
                                    hard_errors.len()
                                );
                                for msg in &hard_errors {
                                    eprintln!("    {}", msg);
                                }
                                for msg in &hard_errors {
                                    file_diagnostics.push(format!("{}: ERROR - {}", member, msg));
                                }
                                // Also report Rule 8 for visibility
                                for msg in &rule8_errors {
                                    eprintln!("    [Rule 8] {}", msg);
                                }
                                for msg in &rule8_errors {
                                    file_diagnostics.push(format!("{}: WARNING - {}", member, msg));
                                }
                                // Warnings (non-error diagnostics)
                                for msg in &result.warnings {
                                    eprintln!("    {}", msg);
                                    file_diagnostics.push(format!("{}: {}", member, msg));
                                }
                                (String::new(), Vec::new(), true)
                            } else {
                                // Rule 8 soft errors — non-blocking
                                if !rule8_errors.is_empty() {
                                    eprintln!(
                                        "  '{}': {} Rule 8 diagnostic(s) (non-blocking)",
                                        member,
                                        rule8_errors.len()
                                    );
                                    for msg in &rule8_errors {
                                        eprintln!("    {}", msg);
                                    }
                                    for msg in &rule8_errors {
                                        file_diagnostics
                                            .push(format!("{}: WARNING - {}", member, msg));
                                    }
                                }
                                // Other warnings
                                if !result.warnings.is_empty() {
                                    eprintln!(
                                        "  '{}': {} diagnostic(s)",
                                        member,
                                        result.warnings.len()
                                    );
                                    for msg in &result.warnings {
                                        eprintln!("    {}", msg);
                                    }
                                    for msg in &result.warnings {
                                        file_diagnostics.push(format!("{}: {}", member, msg));
                                    }
                                }

                                let cabi_exports = result.cabi_exports;

                                // Collect var_types for test code generation
                                if is_test_group {
                                    let test_cases =
                                        crate::testgen::extract_test_cases(program, &src);
                                    let ret_type_map: HashMap<String, String> = result
                                        .var_types
                                        .iter()
                                        .map(|(k, v)| (k.clone(), v.to_zig_type()))
                                        .collect();
                                    let file_test_code = crate::testgen::generate_test_code(
                                        &test_cases,
                                        &HashSet::new(), // closure_fns not supported yet
                                        &ret_type_map,
                                    );
                                    all_test_code.push_str(&file_test_code);
                                }

                                (result.zig_code, cabi_exports, false)
                            }
                        }
                        Err(e) => {
                            eprintln!("  skip '{}': transpile error", member);
                            file_diagnostics.push(format!("{}: ERROR - {}", member, e));
                            (String::new(), Vec::new(), true)
                        }
                    };

                    if has_error {
                        continue;
                    }

                    // Collect export names from exports_for_all_modules (JS-level) +
                    // cabi_exports (IR-level) for module re-export mapping.
                    for exp in &exports_for_all_modules {
                        all_module_exports.push((exp.clone(), module_name.clone()));
                    }
                    // Also add names from cabi_exports that aren't in exports_for_all_modules
                    // (e.g. functions generated by the IR that weren't in the JS export set).
                    for ce in &cabi_exports {
                        if !exports_for_all_modules.contains(&ce.name) {
                            all_module_exports.push((ce.name.clone(), module_name.clone()));
                        }
                    }

                    let dep_imports = build_dep_imports(member, &group);

                    per_file_modules.push(crate::project::PerFileModule {
                        mod_name: module_name.clone(),
                        zig_code,
                        dep_imports,
                    });

                    // Always collect CABI exports for all groups (paired with module name)
                    for export in cabi_exports {
                        all_cabi_exports.push((module_name.clone(), export));
                    }
                }

                // Only skip the group if NO files succeeded transpilation.
                // Individual file errors are logged above; successful files still get compiled.
                if per_file_modules.is_empty() {
                    if has_error {
                        eprintln!("  skip: all files failed transpilation");
                    } else {
                        eprintln!("  skip: no valid modules after transpilation");
                    }
                    break 'group_block;
                }

                if has_error {
                    eprintln!(
                        "  warning: {} file(s) had transpile errors, continuing with {} successful file(s)",
                        group.members.len() - per_file_modules.len(),
                        per_file_modules.len()
                    );
                }

                // --- Detect export name collisions and build rename map ---
                // If two modules export a function with the same bare name,
                // disambiguate by appending _{module_name} to the CABI/public name.
                let bare_name_counts: HashMap<&str, usize> = {
                    let mut counts: HashMap<&str, usize> = HashMap::new();
                    for (exp_name, _) in &all_module_exports {
                        *counts.entry(exp_name.as_str()).or_insert(0) += 1;
                    }
                    counts
                };
                let is_colliding =
                    |name: &str| -> bool { bare_name_counts.get(name).copied().unwrap_or(0) > 1 };

                // cabi_rename: maps cabi_name (possibly disambiguated) → bare_name
                // name_to_module: maps cabi_name → source module name
                let mut cabi_rename: HashMap<String, String> = HashMap::new();
                let mut name_to_module: HashMap<String, String> = HashMap::new();
                for (exp_name, mod_name) in &all_module_exports {
                    let cabi_name = disambiguate_name(exp_name, mod_name, |n| is_colliding(n));
                    cabi_rename
                        .entry(cabi_name.clone())
                        .or_insert_with(|| exp_name.clone());
                    name_to_module
                        .entry(cabi_name)
                        .or_insert_with(|| mod_name.clone());
                }
                let mut name_to_cabi: HashMap<String, &crate::types::NativeCabiExport> =
                    HashMap::new();
                for (mod_name, exp) in &all_cabi_exports {
                    let key = disambiguate_name(&exp.name, mod_name, |n| is_colliding(n));
                    name_to_cabi.entry(key).or_insert(exp);
                }
                let cabi_wrapper_code =
                    gen_cabi_wrappers(&name_to_module, &name_to_cabi, &cabi_rename);
                let cabi_names: HashSet<String> = name_to_cabi.keys().cloned().collect();

                // Build disambiguated external_exports: Vec<(cabi_name, module_name, bare_name)>
                let disambiguated_exports: Vec<(String, String, String)> = all_module_exports
                    .iter()
                    .map(|(exp_name, mod_name)| {
                        let cabi_name = disambiguate_name(exp_name, mod_name, |n| is_colliding(n));
                        (cabi_name, mod_name.clone(), exp_name.clone())
                    })
                    .collect();

                let project_opts = crate::project::ProjectOptions {
                    name: group.core_name.clone(),
                    out_dir: out_dir.clone(),
                    per_file_code: per_file_modules,
                    external_exports: disambiguated_exports,
                    cabi_wrapper_code,
                    cabi_names,
                    test_code: all_test_code,
                    runtime_dir: Some(runtime_dir.clone()),
                    host_header: if combined_zig.contains("host.")
                        || !async_host_fn_names.is_empty()
                    {
                        host_header.clone()
                    } else {
                        String::new()
                    },
                    async_host_fn_names: async_host_fn_names.clone(),
                    include_windows_stub: group_idx == 0,
                    export_rename: cabi_rename.clone(),
                };

                match crate::project::generate(&project_opts) {
                    Ok(()) => {
                        if verbose {
                            println!("  Generated: {}/{}", out_dir, group.core_name);
                        }
                    }
                    Err(e) => {
                        eprintln!("  FAIL ({})", e);
                        break 'group_block;
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
                        let _ = fs::write(&sm_path, json_str);
                    }
                }

                // Write CABI metadata for all groups
                // Only include init/deinit for the first non-test group
                let include_init = !is_test_group && group_idx == 0;
                write_cabi_metadata(
                    Path::new(&out_dir),
                    &group.core_name,
                    &all_cabi_exports,
                    &host_fns,
                    include_init,
                    &cabi_rename,
                );

                // Collect cabi_exports_json for the result
                let cabi_path = Path::new(&out_dir)
                    .join(&group.core_name)
                    .join("cabi_exports.json");
                let cabi_json = fs::read_to_string(&cabi_path).unwrap_or_default();

                group_results.push(crate::GroupResult {
                    name: group.core_name.clone(),
                    is_test: is_test_group,
                    cabi_exports_json: cabi_json,
                    diagnostics: file_diagnostics.clone(),
                });

                // Persist diagnostics so cache-hit builds can reload them
                // instead of returning empty and causing spurious cargo:warning replay.
                save_diagnostics(Path::new(&out_dir), &group.core_name, &file_diagnostics);

                // Write test_cases.json for test groups (used by bridge test generation)
            }

            // === Zig build ===
            if !config.run_zig_build {
                // Skip zig build/test — caller handles compilation (e.g. proc-macro)
                break 'group_block;
            }

            let project_path = Path::new(&out_dir).join(&group.core_name);
            let mut build_ok = false;
            let mut build_cmd = Command::new("zig");
            build_cmd.arg("build");
            if let Some(ref opt) = config.zig_optimize {
                build_cmd.arg(format!("-Doptimize={}", opt));
            }
            let build_result = build_cmd.current_dir(&project_path).output();
            match build_result {
                Ok(result) if result.status.success() => {
                    if verbose {
                        println!("  zig build: OK");
                    }
                    build_ok = true;
                }
                Ok(result) => {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    eprintln!("  zig build FAILED:\n{}", stderr);
                    return Err(format!(
                        "zig build failed for group '{}':\n{}",
                        group.core_name,
                        stderr.lines().take(20).collect::<Vec<_>>().join("\n")
                    ));
                }
                Err(_) => {
                    eprintln!("  warning: zig not found — skipping build");
                }
            }

            // === Zig tests ===
            // Skip zig test when host functions are present — they require Rust-side
            // symbol resolution and cannot be linked by zig test standalone.
            // Check both: (1) registered host_fns from toml, and (2) host.zig with
            // extern declarations (auto-injected for regex etc.).
            let has_host_zig = project_path.join("src").join("host.zig").exists();
            let has_host_deps = !host_fns.is_empty() || has_host_zig;
            let mut test_ok = false;
            if has_host_deps {
                if verbose {
                    println!("  zig test: SKIPPED (project has host function dependencies)");
                }
                test_ok = true; // don't block cache update
            } else {
                let test_result = Command::new("zig")
                    .arg("build")
                    .arg("test")
                    .current_dir(&project_path)
                    .output();
                match test_result {
                    Ok(result) if result.status.success() => {
                        if verbose {
                            println!("  zig test: PASSED");
                        }
                        test_ok = true;
                    }
                    Ok(result) => {
                        let stderr = String::from_utf8_lossy(&result.stderr);
                        eprintln!("  zig test FAILED:\n{}", stderr);
                    }
                    Err(_) => {}
                }
            }

            // === Update build cache on success ===
            if build_ok && test_ok {
                build_cache.insert(group.core_name.clone(), current_hash.clone());
            }
        } // else (re-transpile path)
    }

    // === Write build cache ===
    write_build_cache(Path::new(&out_dir), &build_cache);

    let all_diagnostics: Vec<String> = group_results
        .iter()
        .flat_map(|g| g.diagnostics.clone())
        .collect();

    Ok(ProjectResult {
        groups: group_results,
        diagnostics: all_diagnostics,
    })
}

/// Generate `pub export fn` wrapper code for lib.zig.
/// Each wrapper calls the per-file module function and lives in the root lib.zig,
/// so Zig correctly propagates the symbols into the final .lib.
///
/// For string-returning functions, ALSO generate a Zig-friendly adapter
/// (`pub fn greet(s: []const u8) []const u8`) so test code can call
/// the function with idiomatic Zig string types.
///
/// `cabi_rename` maps disambiguated CABI names → bare function names.
/// Format conversion statements: empty → empty string, otherwise joined with newlines.
fn format_conversions(convs: &[String]) -> String {
    if convs.is_empty() {
        String::new()
    } else {
        format!("{}\n", convs.join("\n"))
    }
}

/// Emit `comptime { @export(...) }` line for a CABI wrapper.
fn emit_comptime_export(out: &mut String, name: &str) {
    out.push_str(&format!(
        "comptime {{ @export(&{name}_cabi, .{{ .name = \"{name}\", .linkage = .strong }}); }}\n",
        name = name,
    ));
}

fn emit_const_alias(out: &mut String, name: &str, bare_name: &str, module: &str) {
    out.push_str(&format!(
        "pub const {name} = {mod}.{bare};\n\n",
        name = name,
        bare = bare_name,
        mod = module,
    ));
}

/// Disambiguate a name by appending the module name if it collides.
fn disambiguate_name(name: &str, module: &str, is_colliding: impl Fn(&str) -> bool) -> String {
    if is_colliding(name) {
        format!("{}_{}", name, module)
    } else {
        name.to_string()
    }
}

/// Emit an async Zig-friendly adapter function.
fn emit_async_adapter(
    out: &mut String,
    name: &str,
    bare: &str,
    params: &str,
    ret_type: &str,
    module: &str,
    args: &str,
) {
    out.push_str(&format!(
        "pub fn {name}({params}) {ret_type} {{\n    return {module}.{bare}({args}) catch @panic(\"async error in {name}\");\n}}\n",
        name = name,
        bare = bare,
        params = params,
        ret_type = ret_type,
        module = module,
        args = args,
    ));
}

/// When an export name collides across modules, the CABI wrapper gets the
/// disambiguated name (`{fn}_{module}`) as its public symbol, but calls the
/// original bare-named function inside the per-file module.
pub fn gen_cabi_wrappers(
    name_to_module: &HashMap<String, String>,
    name_to_cabi: &HashMap<String, &crate::types::NativeCabiExport>,
    cabi_rename: &HashMap<String, String>,
) -> String {
    use std::collections::HashSet;

    let mut out = String::new();
    let mut emitted: HashSet<&str> = HashSet::new();

    for (cabi_name, exp) in name_to_cabi {
        if !emitted.insert(cabi_name.as_str()) {
            continue;
        }
        let Some(module) = name_to_module.get(cabi_name) else {
            continue;
        };
        // Prefix module name with _ to match orchestrator import (const _mod = @import(...))
        let module = format!("_{}", module);
        // Bare function name inside the per-file module (may differ from cabi_name when collision)
        let bare_name = cabi_rename
            .get(cabi_name)
            .map(|s| s.as_str())
            .unwrap_or(cabi_name.as_str());
        // `name` = public/disambiguated name (used for wrapper declarations, @export, etc.)
        let name = cabi_name.as_str();

        let returns_string = exp.ret_type == crate::types::ZigType::Str;
        let ret_is_js_any = exp.ret_type == crate::types::ZigType::Anytype;
        let ret_is_arraylist = matches!(exp.ret_type, crate::types::ZigType::ArrayList(_));

        // JsAny/ArrayList returns: re-export as const alias (no CABI export).
        // This lets Zig test code call the function, but no C ABI symbol is emitted.
        if ret_is_js_any || ret_is_arraylist {
            emit_const_alias(&mut out, name, bare_name, &module);
            continue;
        }

        // Skip functions with JsValue/JsAny parameters (C ABI doesn't support unions)
        let has_js_obj_param = exp.params.iter().any(|(_, ty)| {
            *ty == crate::types::ZigType::Void || *ty == crate::types::ZigType::Anytype
        });
        if has_js_obj_param {
            emit_const_alias(&mut out, name, bare_name, &module);
            continue;
        }

        // Build parameter lists for all function types
        let mut cabi_params: Vec<String> = Vec::new();
        let mut zig_params: Vec<String> = Vec::new();
        let mut arg_names: Vec<String> = Vec::new();
        let mut cabi_to_zig_conversions: Vec<String> = Vec::new();

        for (pname, ptype) in &exp.params {
            arg_names.push(pname.clone());
            if *ptype == crate::types::ZigType::Str {
                cabi_params.push(format!("{}: [*:0]const u8", pname));
                zig_params.push(format!("{}: []const u8", pname));
                cabi_to_zig_conversions.push(format!(
                    "    const {p}_slice: []const u8 = std.mem.span({p});",
                    p = pname
                ));
            } else {
                let zig_ty = ptype.to_zig_type();
                cabi_params.push(format!("{}: {}", pname, zig_ty));
                zig_params.push(format!("{}: {}", pname, zig_ty));
            }
        }

        // Build call args: for CABI wrapper, string params use _slice version
        let zig_call_args: String = arg_names.join(", ");
        let cabi_call_args: String = exp
            .params
            .iter()
            .map(|(pname, ptype)| {
                if *ptype == crate::types::ZigType::Str {
                    format!("{}_slice", pname)
                } else {
                    pname.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

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
                emit_async_adapter(
                    &mut out,
                    name,
                    bare_name,
                    &zig_params.join(", "),
                    "[]const u8",
                    &module,
                    &async_zig_args,
                );
                // C ABI wrapper (StrRet — zero-copy, panic via negative len)
                let conversions = format_conversions(&cabi_to_zig_conversions);
                out.push_str(&format!(
                    "pub export fn {name}_cabi({cabi_params}) StrRet {{\n{conv}    return StrRet.from({mod}.{bare}({args}) catch |err| return StrRet.from_panic(err));\n}}\n",
                    name = name,
                    bare = bare_name,
                    cabi_params = cabi_params.join(", "),
                    conv = conversions,
                    mod = module,
                    args = async_cabi_args,
                ));
                emit_comptime_export(&mut out, name);
            } else if let crate::types::ZigType::NamedStruct(ref sn) = exp.ret_type {
                // Async struct return: use out-pointer C ABI wrapper
                let struct_name = format!("host.{}", sn);
                let conversions = format_conversions(&cabi_to_zig_conversions);

                // Zig-friendly adapter (for tests)
                emit_async_adapter(
                    &mut out,
                    name,
                    bare_name,
                    &zig_params.join(", "),
                    &struct_name,
                    &module,
                    &async_zig_args,
                );

                // C ABI wrapper: add *<struct_name> out-pointer parameter
                let mut cabi_params_with_out = cabi_params.clone();
                cabi_params_with_out.push(format!("result: *{}", struct_name));
                let cabi_params_str = cabi_params_with_out.join(", ");

                let cabi_call = format!(
                    "{mod}.{bare}({args})",
                    mod = module,
                    bare = bare_name,
                    args = async_cabi_args,
                );
                out.push_str(&format!(
                    "pub export fn {name}_cabi({params}) void {{\n{conv}    const _result = {cabi_call} catch @panic(\"async error in {name}\");\n    result.* = _result;\n}}\n",
                    name = name,
                    params = cabi_params_str,
                    conv = conversions,
                    cabi_call = cabi_call,
                ));
                emit_comptime_export(&mut out, name);
            } else {
                // Async non-string, non-struct return (e.g., i64, bool)
                let ret_zig = exp.ret_type.to_zig_type();
                let conversions = format_conversions(&cabi_to_zig_conversions);

                // Zig-friendly adapter (for tests)
                emit_async_adapter(
                    &mut out,
                    name,
                    bare_name,
                    &zig_params.join(", "),
                    &ret_zig,
                    &module,
                    &async_zig_args,
                );

                // C ABI wrapper
                let cabi_params_with_runtime = {
                    let mut p = cabi_params.clone();
                    p.push("js_runtime: *JSRuntime".to_string());
                    p
                };
                let cabi_params_str = cabi_params_with_runtime.join(", ");

                let cabi_call = format!(
                    "{mod}.{bare}({args})",
                    mod = module,
                    bare = bare_name,
                    args = async_cabi_args,
                );
                out.push_str(&format!(
                    "pub export fn {name}_cabi({params}) {ret} {{\n{conv}    return {cabi_call} catch @panic(\"async error in {name}\");\n}}\n",
                    name = name,
                    params = cabi_params_str,
                    ret = ret_zig,
                    conv = conversions,
                    cabi_call = cabi_call,
                ));
                emit_comptime_export(&mut out, name);
            }

            out.push('\n');
            continue;
        }

        if returns_string {
            // ── Zig-friendly adapter (for tests) — calls _impl directly, no conversion ──
            let test_call = if exp.can_throw {
                format!(
                    "{mod}.{bare}({args}) catch @panic(\"error in {name}\")",
                    mod = module,
                    bare = bare_name,
                    name = name,
                    args = zig_call_args,
                )
            } else {
                format!("{mod}.{bare}({args})", mod = module, bare = bare_name, args = zig_call_args)
            };
            out.push_str(&format!(
                "pub fn {name}({params}) []const u8 {{\n    return {test_call};\n}}\n",
                name = name,
                params = zig_params.join(", "),
                test_call = test_call,
            ));

            // ── C ABI wrapper (StrRet — zero-copy, error via sign-bit) ──
            let conversions = format_conversions(&cabi_to_zig_conversions);
            let cabi_call = if exp.can_throw {
                format!(
                    "{mod}.{bare}({args}) catch |err| return StrRet.from_panic(err)",
                    mod = module,
                    bare = bare_name,
                    args = cabi_call_args,
                )
            } else {
                format!(
                    "{mod}.{bare}({args})",
                    mod = module,
                    bare = bare_name,
                    args = cabi_call_args,
                )
            };
            out.push_str(&format!(
                "pub export fn {name}_cabi({cabi_params}) StrRet {{\n{conv}    return StrRet.from({cabi_call});\n}}\n",
                name = name,
                cabi_params = cabi_params.join(", "),
                conv = conversions,
                cabi_call = cabi_call,
            ));
            emit_comptime_export(&mut out, name);
        } else {
            let ret_zig = exp.ret_type.to_cabi_str();
            let exp_ret_is_js_value = exp.ret_type == crate::types::ZigType::Void;

            // Build C ABI param list: add _err out-param for can_throw non-string exports
            let mut cabi_params_with_err = cabi_params.clone();
            if exp.can_throw {
                cabi_params_with_err.push("err_out: *?[*:0]const u8".to_string());
            }
            let cabi_params_str = cabi_params_with_err.join(", ");

            // Build the call expression with error handling for can_throw
            let (call_expr, err_setup) = if exp.can_throw {
                let err_handle = if ret_zig == "void" {
                    // Void: call without assignment
                    format!(
                        "    {mod}.{bare}({args}) catch |err| {{\n        err_out.* = @errorName(err);\n        return;\n    }};",
                        mod = module,
                        bare = bare_name,
                        args = cabi_call_args,
                    )
                } else {
                    // Use type-appropriate zero value for the catch fallback
                    let ret_zero = if ret_zig == "bool" { "false" } else { "0" };
                    format!(
                        "    const _result = {mod}.{bare}({args}) catch |err| {{\n        err_out.* = @errorName(err);\n        return {ret_zero};\n    }};",
                        mod = module,
                        bare = bare_name,
                        args = cabi_call_args,
                        ret_zero = ret_zero,
                    )
                };
                let setup = if ret_zig == "void" {
                    "    err_out.* = null;\n".to_string()
                } else {
                    String::new()
                };
                (err_handle, setup)
            } else {
                (
                    format!(
                        "{mod}.{bare}({args})",
                        mod = module,
                        bare = bare_name,
                        args = cabi_call_args,
                    ),
                    String::new(),
                )
            };

            let conversions = format_conversions(&cabi_to_zig_conversions);

            if ret_zig == "void" {
                if exp.can_throw {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) void {{\n{conv}{err_setup}{call_expr}\n}}\n",
                        name = name,
                        params = cabi_params_str,
                        conv = conversions,
                        err_setup = err_setup,
                        call_expr = call_expr,
                    ));
                } else {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) void {{\n{conv}    {mod}.{bare}({args});\n}}\n",
                        name = name,
                        bare = bare_name,
                        params = cabi_params_str,
                        conv = conversions,
                        mod = module,
                        args = cabi_call_args,
                    ));
                }
            } else if exp_ret_is_js_value {
                // JsValue: extract .int for C ABI (i64)
                if exp.can_throw {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) i64 {{\n{conv}{call_expr}\n    return _result.int;\n}}\n",
                        name = name,
                        params = cabi_params_str,
                        conv = conversions,
                        call_expr = call_expr,
                    ));
                } else {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) i64 {{\n{conv}    const _result = {mod}.{bare}({args});\n    return _result.int;\n}}\n",
                        name = name,
                        bare = bare_name,
                        params = cabi_params_str,
                        conv = conversions,
                        mod = module,
                        args = cabi_call_args,
                    ));
                }
            } else {
                // Use type-appropriate zero value for void fallback
                let rz = if ret_zig == "bool" { "false" } else { "0" };
                if exp.can_throw {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) {ret} {{\n{conv}{call_expr}\n    if (@TypeOf(_result) == void) {{\n        return {rz};\n    }} else {{\n        return _result;\n    }}\n}}\n",
                        name = name,
                        params = cabi_params_str,
                        conv = conversions,
                        ret = ret_zig,
                        call_expr = call_expr,
                        rz = rz,
                    ));
                } else {
                    out.push_str(&format!(
                        "pub export fn {name}({params}) {ret} {{\n{conv}    const _result = {mod}.{bare}({args});\n    if (@TypeOf(_result) == void) {{\n        return {rz};\n    }} else {{\n        return _result;\n    }}\n}}\n",
                        name = name,
                        bare = bare_name,
                        params = cabi_params_str,
                        ret = ret_zig,
                        conv = conversions,
                        mod = module,
                        args = cabi_call_args,
                        rz = rz,
                    ));
                }
            }
        }

        out.push('\n');
    }

    out
}

/// Write C ABI exports/imports JSON metadata for a single group project.
///
/// `cabi_rename` maps disambiguated CABI names → bare function names.
/// When a name collides across modules, the JSON "name" field uses the
/// Write C ABI exports/imports JSON metadata for a single group project.
///
/// `cabi_exports` is a list of (module_name, export) pairs.
/// `cabi_rename` maps disambiguated CABI names → bare function names.
/// When a name collides across modules, the JSON "name" field uses the
/// disambiguated form (`{fn}_{module}`) so that the bridge macro generates
/// unique Rust function definitions.
pub fn write_cabi_metadata(
    out_dir: &Path,
    group_name: &str,
    cabi_exports: &[(String, crate::types::NativeCabiExport)],
    host_fns: &crate::host::HostFnRegistry,
    include_init: bool,
    cabi_rename: &HashMap<String, String>,
) {
    let project_dir = out_dir.join(group_name);

    // cabi_exports.json — filter out exports with Anytype returns or params (no C ABI export generated)
    let exports_path = project_dir.join("cabi_exports.json");
    let mut exports_value: Vec<serde_json::Value> = cabi_exports
        .iter()
        .filter(|(_, exp)| {
            exp.ret_type != crate::types::ZigType::Anytype
                && !exp
                    .params
                    .iter()
                    .any(|(_, ty)| *ty == crate::types::ZigType::Anytype)
        })
        .map(|(mod_name, exp)| {
            // Build params list
            let params: Vec<serde_json::Value> = exp
                .params
                .iter()
                .map(|(name, ty)| {
                    serde_json::json!({
                        "name": name,
                        "zig_type": ty.to_zig_type() // JSON metadata is consumed by macro (lib.zig)
                    })
                })
                .collect();

            // Determine ret_type string for C ABI
            let ret_type_str = exp.ret_type.to_cabi_str();

            // For NamedStruct returns, look up struct fields from host_fns
            let (ret_struct_name, ret_struct_fields) =
                if let crate::types::ZigType::NamedStruct(ref struct_name) = exp.ret_type {
                    // Look up the struct definition from host_fns
                    let struct_fields: Option<Vec<serde_json::Value>> = host_fns
                        .structs
                        .iter()
                        .find(|s| &s.zig_name == struct_name)
                        .map(|s| {
                            s.fields
                                .iter()
                                .map(|f| {
                                    serde_json::json!({
                                        "name": f.name,
                                        "zig_type": f.zig_type,
                                        "cabi_type": f.c_type,
                                    })
                                })
                                .collect()
                        });
                    (Some(struct_name.clone()), struct_fields)
                } else {
                    (None, None)
                };

            // Use disambiguated name if this export collides across modules
            let disambiguated = format!("{}_{}", exp.name, mod_name);
            let export_name = if cabi_rename.contains_key(&disambiguated) {
                disambiguated
            } else {
                exp.name.clone()
            };

            let mut json_obj = serde_json::json!({
                "name": export_name,
                "params": params,
                "ret_type": ret_type_str,
                "can_throw": exp.can_throw,
            });

            // Add struct info if returning a NamedStruct
            if let Some(sn) = ret_struct_name {
                json_obj["ret_struct_name"] = serde_json::json!(sn);
            }
            if let Some(sf) = ret_struct_fields {
                json_obj["ret_struct_fields"] = serde_json::json!(sf);
            }

            json_obj
        })
        .collect();

    // Deduplicate exports by name — when multiple JS files produce identically-named
    // C ABI exports, the bridge macro would generate duplicate Rust function definitions,
    // causing E0428 compilation errors. With collision disambiguation, duplicates are
    // resolved by the {fn}_{module} naming, but we still deduplicate as a safety net.
    {
        let mut seen = HashSet::new();
        exports_value.retain(|exp| {
            let name = exp["name"].as_str().unwrap_or("");
            seen.insert(name.to_string())
        });
    }

    // Only include js2rust_init and js2rust_deinit for the first non-test group
    if include_init {
        exports_value.push(serde_json::json!({
            "name": "js2rust_init",
            "params": [],
            "ret_type": "void",
        }));
        exports_value.push(serde_json::json!({
            "name": "js2rust_deinit",
            "params": [],
            "ret_type": "void",
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
