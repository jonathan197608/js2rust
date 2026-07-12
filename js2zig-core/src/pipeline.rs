use crate::analyzer::{analyze_project, sanitize_module_name};
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
/// Given a filename like "main.js", looks up its imported_names in the analysis
/// and maps source filenames to sanitized Zig module names.
fn build_dep_imports(
    filename: &str,
    analysis: &crate::analyzer::AnalysisResult,
) -> Vec<(String, String)> {
    let empty = Vec::new();
    let raw_imports = analysis.imported_names.get(filename).unwrap_or(&empty);
    raw_imports
        .iter()
        .map(|(imported_name, src_file)| {
            let mod_name = analysis
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

/// Compute a content hash for the project.
/// Hashes all member JS files + runtime .zig files so any change triggers rebuild.
fn compute_content_hash(
    in_dir: &Path,
    analysis: &crate::analyzer::AnalysisResult,
    runtime_dir: &Path,
) -> String {
    let mut hasher = std::hash::DefaultHasher::new();

    // Hash each member JS file content (sorted for determinism)
    let mut members: Vec<&String> = analysis.members.iter().collect();
    members.sort();
    for member in &members {
        member.hash(&mut hasher);
        if let Ok(content) = fs::read(in_dir.join(member)) {
            content.hash(&mut hasher);
        }
    }

    // Hash runtime .zig files (changes here affect the output)
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
/// Returns project_name → hash_hex map.
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

/// Save project diagnostics to `out/<project_name>/diagnostics.json`.
/// On cache hit, these are loaded back so the caller always sees
/// consistent diagnostic output without re-transpiling.
fn save_diagnostics(out_dir: &Path, project_name: &str, diagnostics: &[String]) {
    let path = Path::new(out_dir)
        .join(project_name)
        .join("diagnostics.json");
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

    // === Phase 1: Analyze project (entry file + transitive deps) ===
    let additional_js_files: Vec<String> = config
        .additional_roots
        .iter()
        .filter_map(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .collect();
    let analysis = analyze_project(&in_dir, &core_file, &additional_js_files);

    // Emit cargo:rerun-if-changed for every JS file discovered by the analyzer
    // (including transitive dependencies not listed in js2rust.toml).
    // These directives take effect in subsequent builds — Cargo stores them
    // and uses them to decide whether to re-run the build script.
    // Only emit when called from build.rs (is_build_script=true);
    // proc-macros cannot use these directives and their stdout would leak
    // noise to the terminal.
    if config.is_build_script {
        for member in &analysis.members {
            let member_path = Path::new(&in_dir).join(member);
            println!("cargo:rerun-if-changed={}", member_path.display());
        }
    }

    if analysis.members.is_empty() {
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
    }

    let host_header = host_fns.generate_zig_header();
    let async_host_fn_names: Vec<String> = host_fns.async_fn_names();
    let runtime_dir = ws.join("runtime").to_string_lossy().to_string();

    // === Incremental compilation: load build cache ===
    let mut build_cache = read_build_cache(Path::new(&out_dir));
    let runtime_path = ws.join("runtime");
    let project_name = analysis.core_name.clone();
    let is_test = project_name.starts_with("test_");

    // === Phase 2: Generate Zig project ===
    // Always uses multi-file mode: one .zig per JS file + orchestrator lib.zig.
    // Returns (diagnostics, cabi_exports_json, updated_build_cache).
    let (result_diagnostics, result_cabi_json, build_cache) = {
        #[allow(unused_assignments)] // initial values overwritten in both branches below
        let mut result_diagnostics: Vec<String> = Vec::new();
        #[allow(unused_assignments)]
        let mut result_cabi_json = String::new();
        if verbose {
            println!(
                "\n=== {} ({} member{}) {}===",
                project_name,
                analysis.members.len(),
                if analysis.members.len() == 1 { "" } else { "s" },
                if is_test { "[test] " } else { "" }
            );
        }

        // --- Incremental check ---
        let current_hash = compute_content_hash(&in_path, &analysis, &runtime_path);
        if !force_rebuild
            && let Some(cached_hash) = build_cache.get(&project_name)
            && *cached_hash == current_hash
        {
            if verbose {
                println!("  unchanged (cache hit)");
            }
            let cabi_path = Path::new(&out_dir)
                .join(&project_name)
                .join("cabi_exports.json");
            result_cabi_json = fs::read_to_string(&cabi_path).unwrap_or_default();

            // Load cached diagnostics so the caller always sees consistent output.
            let diag_path = Path::new(&out_dir)
                .join(&project_name)
                .join("diagnostics.json");
            result_diagnostics = fs::read_to_string(&diag_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
        } else {
            // Hash mismatch (or force_rebuild) — re-transpile.
            if force_rebuild {
                if verbose {
                    println!("  force rebuild");
                }
            } else if verbose {
                println!("  source changed, re-transpiling");
            }

            let mut per_file_modules: Vec<crate::project::PerFileModule> = Vec::new();
            let mut all_module_exports: Vec<(String, String)> = Vec::new();
            let mut all_test_code = String::new();
            let mut all_cabi_exports: Vec<(String, crate::types::NativeCabiExport)> = Vec::new();
            let mut file_diagnostics: Vec<String> = Vec::new();

            // --- Transpile pass (all metadata from analysis, no source scanning) ---
            let core_exports = analysis
                .exported_names
                .get(&analysis.core_file)
                .cloned()
                .unwrap_or_default();

            // --- Compute re-exported names per dependency ---
            let core_imports = analysis
                .imported_names
                .get(&analysis.core_file)
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

            let mut has_any_error = false;

            for member in &analysis.members {
                let src = match analysis.file_sources.get(member) {
                    Some(s) => s.as_str(),
                    None => {
                        eprintln!("  skip '{}': no cached source", member);
                        continue;
                    }
                };

                if src.trim().is_empty() {
                    eprintln!("  skip '{}': empty source", member);
                    continue;
                }

                let module_name = analysis.name_map.get(member).cloned().unwrap_or_else(|| {
                    let stem = member.strip_suffix(".js").unwrap_or(member);
                    sanitize_module_name(stem)
                });

                // For test projects: ALL toplevel functions → C ABI.
                // For normal projects: entry file's JS exports → C ABI;
                //                     additional core files → C ABI (full exports);
                //                     dependency file: only re-exported names → C ABI.
                let transpile_exports: HashSet<String> = if is_test {
                    analysis
                        .all_fn_names
                        .get(member)
                        .cloned()
                        .unwrap_or_default()
                } else if *member == analysis.core_file || additional_core_set.contains(member) {
                    analysis
                        .exported_names
                        .get(member)
                        .cloned()
                        .unwrap_or_default()
                } else {
                    dep_re_exports.get(member).cloned().unwrap_or_default()
                };

                let module_exports = transpile_exports.clone();

                let program = match analysis.parsed_programs.get(member) {
                    Some(p) => p,
                    None => {
                        eprintln!("  skip '{}': no parsed program", member);
                        continue;
                    }
                };

                let transpile_result = crate::native_proto::transpile_js(
                    program,
                    src,
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
                                    file_diagnostics.push(format!("{}: WARNING - {}", member, msg));
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
                            if is_test {
                                let test_cases = crate::testgen::extract_test_cases(program, src);
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
                    has_any_error = true;
                    continue;
                }

                // Collect export names from module_exports (JS-level) +
                // cabi_exports (IR-level) for module re-export mapping.
                for exp in &module_exports {
                    all_module_exports.push((exp.clone(), module_name.clone()));
                }
                // Also add names from cabi_exports that aren't in module_exports
                // (e.g. functions generated by the IR that weren't in the JS export set).
                for ce in &cabi_exports {
                    if !module_exports.contains(&ce.name) {
                        all_module_exports.push((ce.name.clone(), module_name.clone()));
                    }
                }

                let dep_imports = build_dep_imports(member, &analysis);

                per_file_modules.push(crate::project::PerFileModule {
                    mod_name: module_name.clone(),
                    zig_code,
                    dep_imports,
                });

                // Always collect CABI exports (paired with module name)
                for export in cabi_exports {
                    all_cabi_exports.push((module_name.clone(), export));
                }
            }

            // Only fail if NO files succeeded transpilation.
            // Individual file errors are logged above; successful files still get compiled.
            if per_file_modules.is_empty() {
                if has_any_error {
                    eprintln!("  skip: all files failed transpilation");
                } else {
                    eprintln!("  skip: no valid modules after transpilation");
                }
                // Write build cache and return empty result
                write_build_cache(Path::new(&out_dir), &build_cache);
                return Ok(ProjectResult {
                    project_name,
                    is_test,
                    cabi_exports_json: String::new(),
                    diagnostics: file_diagnostics,
                });
            }

            if has_any_error {
                eprintln!(
                    "  warning: {} file(s) had transpile errors, continuing with {} successful file(s)",
                    analysis.members.len() - per_file_modules.len(),
                    per_file_modules.len()
                );
            }

            // --- Detect export name collisions and build rename map ---
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
            let mut name_to_cabi: HashMap<String, &crate::types::NativeCabiExport> = HashMap::new();
            for (mod_name, exp) in &all_cabi_exports {
                let key = disambiguate_name(&exp.name, mod_name, |n| is_colliding(n));
                name_to_cabi.entry(key).or_insert(exp);
            }
            let cabi_wrapper_code = gen_cabi_wrappers(&name_to_module, &name_to_cabi, &cabi_rename);
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
                name: project_name.clone(),
                out_dir: out_dir.clone(),
                per_file_code: per_file_modules,
                external_exports: disambiguated_exports,
                cabi_wrapper_code,
                cabi_names,
                test_code: all_test_code,
                runtime_dir: Some(runtime_dir.clone()),
                host_header: if !host_fns.is_empty() {
                    host_header.clone()
                } else {
                    String::new()
                },
                async_host_fn_names: async_host_fn_names.clone(),
            };

            match crate::project::generate(&project_opts) {
                Ok(()) => {
                    if verbose {
                        println!("  Generated: {}/{}", out_dir, project_name);
                    }
                }
                Err(e) => {
                    eprintln!("  FAIL ({})", e);
                    write_build_cache(Path::new(&out_dir), &build_cache);
                    return Ok(ProjectResult {
                        project_name,
                        is_test,
                        cabi_exports_json: String::new(),
                        diagnostics: file_diagnostics,
                    });
                }
            }

            // Write CABI metadata — always include init/deinit for non-test projects
            let include_init = !is_test;
            write_cabi_metadata(
                Path::new(&out_dir),
                &project_name,
                &all_cabi_exports,
                &host_fns,
                include_init,
                &cabi_rename,
            );

            // Collect cabi_exports_json for the result
            let cabi_path = Path::new(&out_dir)
                .join(&project_name)
                .join("cabi_exports.json");
            result_cabi_json = fs::read_to_string(&cabi_path).unwrap_or_default();

            // Persist diagnostics so cache-hit builds can reload them
            // instead of returning empty and causing spurious cargo:warning replay.
            save_diagnostics(Path::new(&out_dir), &project_name, &file_diagnostics);
            result_diagnostics = file_diagnostics;

            // === Zig build ===
            if config.run_zig_build {
                let project_path = Path::new(&out_dir).join(&project_name);
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
                            "zig build failed for project '{}':\n{}",
                            project_name,
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
                    build_cache.insert(project_name.clone(), current_hash.clone());
                }
            }
        }
        (result_diagnostics, result_cabi_json, build_cache)
    };

    // === Write build cache ===
    write_build_cache(Path::new(&out_dir), &build_cache);

    Ok(ProjectResult {
        project_name,
        is_test,
        cabi_exports_json: result_cabi_json,
        diagnostics: result_diagnostics,
    })
}

use crate::cabi::{disambiguate_name, gen_cabi_wrappers, write_cabi_metadata};
