use crate::analyzer::{analyze_project, sanitize_module_name};
use crate::{ProjectConfig, ProjectResult};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

    // js_dir is the source directory; js_files[0] is the primary entry.
    let in_dir = &config.js_dir;
    let js_files = &config.js_files;
    if js_files.is_empty() {
        return Err("js2rust_bridge: project.js_files is empty in js2rust.toml".to_string());
    }
    let primary_root = &js_files[0];
    let out_dir = config.out_dir.clone();
    let force_rebuild = config.build.force_rebuild;
    let verbose = config.build.is_build_script; // only print progress in build.rs context

    // Ensure output directory exists.
    fs::create_dir_all(&out_dir).map_err(|e| {
        format!(
            "cannot create output directory '{}': {}",
            out_dir.display(),
            e
        )
    })?;

    // === Phase 1: Analyze project (entry file + transitive deps) ===
    let analysis = analyze_project(in_dir, js_files);

    // Emit cargo:rerun-if-changed for every JS file discovered by the analyzer
    // (including transitive dependencies not listed in js2rust.toml).
    // These directives take effect in subsequent builds — Cargo stores them
    // and uses them to decide whether to re-run the build script.
    // Only emit when called from build.rs (is_build_script=true);
    // proc-macros cannot use these directives and their stdout would leak
    // noise to the terminal.
    if config.build.is_build_script {
        for member in &analysis.members {
            let member_path = in_dir.join(member);
            println!("cargo:rerun-if-changed={}", member_path.display());
        }
    }

    if analysis.members.is_empty() {
        return Err(format!(
            "no JS files discovered from primary root '{}'",
            primary_root
        ));
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
    let runtime_dir = ws.join("runtime");

    // === Incremental compilation: load build cache ===
    let build_cache = read_build_cache(&out_dir);
    let project_name = analysis.entry_name.clone();
    let is_test = project_name.starts_with("test_");

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
    let current_hash = compute_content_hash(in_dir, &analysis, &runtime_dir);
    if let Some((diagnostics, cabi_json)) = try_cache_hit(
        &out_dir,
        &project_name,
        force_rebuild,
        &build_cache,
        &current_hash,
        verbose,
    ) {
        return Ok(ProjectResult {
            project_name,
            is_test,
            cabi_exports_json: cabi_json,
            diagnostics,
        });
    }

    // Cache miss — full transpile + build.
    let (result_diagnostics, result_cabi_json) = transpile_cache_miss(
        &out_dir,
        &project_name,
        is_test,
        &analysis,
        primary_root,
        js_files,
        &host_fns,
        &host_header,
        &async_host_fn_names,
        &current_hash,
        &build_cache,
        config,
        verbose,
        &runtime_dir,
    )?;

    Ok(ProjectResult {
        project_name,
        is_test,
        cabi_exports_json: result_cabi_json,
        diagnostics: result_diagnostics,
    })
}

/// Check the incremental build cache for a matching entry.
///
/// Returns `Some((diagnostics, cabi_json))` on cache hit, or `None` on miss.
fn try_cache_hit(
    out_dir: &Path,
    project_name: &str,
    force_rebuild: bool,
    build_cache: &HashMap<String, String>,
    current_hash: &str,
    verbose: bool,
) -> Option<(Vec<String>, String)> {
    if force_rebuild {
        return None;
    }
    let cached_hash = build_cache.get(project_name)?;
    if *cached_hash != current_hash {
        return None;
    }

    if verbose {
        println!("  unchanged (cache hit)");
    }
    let cabi_path = out_dir.join(project_name).join("cabi_exports.json");
    let cabi_json = fs::read_to_string(&cabi_path).unwrap_or_default();

    // Load cached diagnostics so the caller always sees consistent output.
    let diag_path = out_dir.join(project_name).join("diagnostics.json");
    let diagnostics: Vec<String> = fs::read_to_string(&diag_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    Some((diagnostics, cabi_json))
}

/// Full transpile + build (cache miss path).
///
/// Takes all analysis results and configuration, performs transpilation,
/// writes output files, runs zig build/test, and persists the build cache.
/// Returns `(diagnostics, cabi_json)` on success, or an error string on failure.
#[allow(clippy::too_many_arguments)]
fn transpile_cache_miss(
    out_dir: &Path,
    project_name: &str,
    is_test: bool,
    analysis: &crate::analyzer::AnalysisResult,
    primary_root: &str,
    js_files: &[String],
    host_fns: &crate::host::HostFnRegistry,
    host_header: &str,
    async_host_fn_names: &[String],
    current_hash: &str,
    build_cache: &HashMap<String, String>,
    config: &ProjectConfig,
    verbose: bool,
    runtime_dir: &Path,
) -> Result<(Vec<String>, String), String> {
    if config.build.force_rebuild {
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
        .get(primary_root)
        .cloned()
        .unwrap_or_default();

    // --- Compute re-exported names per dependency ---
    let core_imports = analysis
        .imported_names
        .get(primary_root)
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
        } else if *member == *primary_root || js_files.iter().skip(1).any(|f| *f == *member) {
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
            Some(host_fns),
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
                        file_diagnostics.push(format!("{}: COMPILE_ERROR - {}", member, msg));
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
                    for msg in &rule8_errors {
                        eprintln!("    [Rule 8] {}", msg);
                    }
                    for msg in &rule8_errors {
                        file_diagnostics.push(format!("{}: WARNING - {}", member, msg));
                    }
                    for msg in &result.warnings {
                        eprintln!("    {}", msg);
                        file_diagnostics.push(format!("{}: {}", member, msg));
                    }
                    (String::new(), Vec::new(), true)
                } else {
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
                    if !result.warnings.is_empty() {
                        eprintln!("  '{}': {} diagnostic(s)", member, result.warnings.len());
                        for msg in &result.warnings {
                            eprintln!("    {}", msg);
                        }
                        for msg in &result.warnings {
                            file_diagnostics.push(format!("{}: {}", member, msg));
                        }
                    }

                    let cabi_exports = result.cabi_exports;

                    if is_test {
                        let test_cases = crate::testgen::extract_test_cases(program, src);
                        let ret_type_map: HashMap<String, String> = result
                            .var_types
                            .iter()
                            .map(|(k, v)| (k.clone(), v.to_zig_type().into_owned()))
                            .collect();
                        let file_test_code = crate::testgen::generate_test_code(
                            &test_cases,
                            &HashSet::new(),
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

        for exp in &module_exports {
            all_module_exports.push((exp.clone(), module_name.clone()));
        }
        for ce in &cabi_exports {
            if !module_exports.contains(&ce.name) {
                all_module_exports.push((ce.name.clone(), module_name.clone()));
            }
        }

        let dep_imports = build_dep_imports(member, analysis);

        per_file_modules.push(crate::project::PerFileModule {
            mod_name: module_name.clone(),
            zig_code,
            dep_imports,
        });

        for export in cabi_exports {
            all_cabi_exports.push((module_name.clone(), export));
        }
    }

    // Only fail if NO files succeeded transpilation.
    if per_file_modules.is_empty() {
        if has_any_error {
            eprintln!("  skip: all files failed transpilation");
        } else {
            eprintln!("  skip: no valid modules after transpilation");
        }
        write_build_cache(out_dir, build_cache);
        return Ok((file_diagnostics, String::new()));
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

    let disambiguated_exports: Vec<(String, String, String)> = all_module_exports
        .iter()
        .map(|(exp_name, mod_name)| {
            let cabi_name = disambiguate_name(exp_name, mod_name, |n| is_colliding(n));
            (cabi_name, mod_name.clone(), exp_name.clone())
        })
        .collect();

    // Detect feature usage from emitted Zig code
    let needs_regex = per_file_modules.iter().any(|m| {
        m.zig_code.contains("host_regex.")
            || m.zig_code.contains("js_regexp.")
            || m.zig_code.contains("JsRegExp")
            || m.zig_code.contains("js_string_regex.")
    });

    let needs_icu = per_file_modules.iter().any(|m| {
        m.zig_code.contains("host_icu.")
            || m.zig_code.contains("js_string_icu.localeCompare")
            || m.zig_code.contains("js_string_icu.normalize")
            || m.zig_code.contains("js_string_icu.toLocaleUpper")
            || m.zig_code.contains("js_string_icu.toLocaleLower")
    });

    let project_opts = crate::project::ProjectOptions {
        name: project_name.to_string(),
        out_dir: out_dir.to_path_buf(),
        per_file_code: per_file_modules,
        external_exports: disambiguated_exports,
        cabi_wrapper_code,
        cabi_names,
        test_code: all_test_code,
        runtime_dir: Some(runtime_dir.to_path_buf()),
        host_header: if !host_fns.is_empty() {
            host_header.to_string()
        } else {
            String::new()
        },
        async_host_fn_names: async_host_fn_names.to_vec(),
        needs_regex,
        needs_icu,
    };

    match crate::project::generate(&project_opts) {
        Ok(()) => {
            if verbose {
                println!("  Generated: {}/{}", out_dir.display(), project_name);
            }
        }
        Err(e) => {
            eprintln!("  FAIL ({})", e);
            write_build_cache(out_dir, build_cache);
            return Ok((file_diagnostics, String::new()));
        }
    }

    // Write CABI metadata
    let include_init = !is_test;
    write_cabi_metadata(
        out_dir,
        project_name,
        &all_cabi_exports,
        host_fns,
        include_init,
        &cabi_rename,
    );

    // Collect cabi_exports_json
    let cabi_path = out_dir.join(project_name).join("cabi_exports.json");
    let result_cabi_json = fs::read_to_string(&cabi_path).unwrap_or_default();

    // Persist diagnostics
    save_diagnostics(out_dir, project_name, &file_diagnostics);

    // Update build cache
    let mut build_cache = build_cache.clone();
    build_cache.insert(project_name.to_string(), current_hash.to_string());

    // === Zig build ===
    if config.build.run_zig_build {
        let project_path = out_dir.join(project_name);
        let mut build_cmd = Command::new("zig");
        build_cmd.arg("build");
        if let Some(ref opt) = config.build.zig_optimize {
            build_cmd.arg(format!("-Doptimize={}", opt));
        }
        let build_result = build_cmd.current_dir(&project_path).output();
        match build_result {
            Ok(result) if result.status.success() => {
                if verbose {
                    println!("  zig build: OK");
                }
            }
            Ok(result) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                eprintln!("  zig build FAILED:\n{}", stderr);
                write_build_cache(out_dir, &build_cache);
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
        if !host_fns.is_empty() {
            if verbose {
                println!("  zig test: SKIPPED (project has host function dependencies)");
            }
        } else {
            let test_result = Command::new("zig")
                .arg("build")
                .arg("test")
                .current_dir(&project_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            match test_result {
                Ok(status) if status.success() => {
                    if verbose {
                        println!("  zig test: PASSED");
                    }
                }
                Ok(_) => {
                    eprintln!("  zig test FAILED");
                }
                Err(e) => {
                    eprintln!("  zig test: failed to execute: {}", e);
                }
            }
        }
    }

    // Write build cache on success
    write_build_cache(out_dir, &build_cache);

    Ok((file_diagnostics, result_cabi_json))
}

use crate::cabi::{disambiguate_name, gen_cabi_wrappers, write_cabi_metadata};
