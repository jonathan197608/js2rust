use js2rustc::analyzer::{analyze_groups, sanitize_module_name, strip_imports_extract_exports};
use std::collections::{HashMap, HashSet};
use std::fs;
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
    group: &js2rustc::analyzer::FileGroup,
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

fn main() {
    let ws = workspace_dir();
    let in_dir = ws.join("in").to_string_lossy().to_string();
    let out_dir = ws.join("out").to_string_lossy().to_string();
    let in_path = ws.join("in");

    // Ensure output directory exists.
    fs::create_dir_all(&out_dir).unwrap_or_else(|e| {
        eprintln!("error: cannot create output directory '{}': {}", out_dir, e);
        std::process::exit(1);
    });

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
        eprintln!("error: no core files found in '{}'", in_dir);
        std::process::exit(1);
    }

    // === Load host function registry ===
    let config_path = ws.join("host_config.json");
    let host_fns = if config_path.exists() {
        match js2rustc::host::HostFnRegistry::load_from_file(&config_path) {
            Ok(registry) => registry,
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        eprintln!(
            "warning: '{}' not found — no host functions registered",
            config_path.display()
        );
        js2rustc::host::HostFnRegistry::new()
    };

    let mut builtins = js2rustc::builtins::BuiltinRegistry::new();
    builtins.register_host_fns(&host_fns);
    let host_header = host_fns.generate_zig_header();
    let async_wrappers = host_fns.generate_async_wrappers();
    let runtime_dir = ws.join("runtime").to_string_lossy().to_string();

    // === Phase 2: Generate Zig project per group ===
    // Always uses multi-file mode: one .zig per JS file + orchestrator lib.zig.
    for (group_idx, group) in groups.iter().enumerate() {
        println!(
            "\n=== {} ({} member{}) ===",
            group.core_name,
            group.members.len(),
            if group.members.len() == 1 { "" } else { "s" }
        );

        {
            let mut per_file_modules: Vec<js2rustc::project::PerFileModule> = Vec::new();
            let mut all_module_exports: Vec<(String, String)> = Vec::new();
            let mut all_test_code = String::new();
            let mut combined_zig = String::new();
            let mut all_cabi_exports: Vec<js2rustc::codegen::CabiExport> = Vec::new();
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

                // Core file: its own JS exports → C ABI.
                // Dependency file: only the names re-exported by the core → C ABI.
                let codegen_exports: HashSet<String> =
                    if *member == group.core_file {
                        exports.clone()
                    } else {
                        dep_re_exports.get(member).cloned().unwrap_or_default()
                    };

                let allocator = oxc_allocator::Allocator::default();
                let program = js2rustc::parser::parse(&allocator, stripped);
                let (zig_code, diagnostics, closure_fns, _fn_return_types, cabi_exports) =
                    js2rustc::codegen::generate(&program, &builtins, &codegen_exports);

                let has_file_error = diagnostics
                    .iter()
                    .any(|d| d.kind == js2rustc::infer::DiagnosticKind::Error);
                if has_file_error {
                    let err_count = diagnostics
                        .iter()
                        .filter(|d| d.kind == js2rustc::infer::DiagnosticKind::Error)
                        .count();
                    eprintln!("  skip '{}': {} codegen error(s)", member, err_count);
                    for diag in &diagnostics {
                        if diag.kind == js2rustc::infer::DiagnosticKind::Error {
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

                per_file_modules.push(js2rustc::project::PerFileModule {
                    mod_name: module_name.clone(),
                    zig_code: zig_code.clone(),
                    dep_imports,
                });

                combined_zig.push_str(&zig_code);
                all_cabi_exports.extend(cabi_exports);

                // Testgen: use &stripped so AST span offsets match
                let test_cases = js2rustc::testgen::extract_test_cases(&program, stripped);
                let closure_fn_refs: HashSet<&str> =
                    closure_fns.iter().map(|s| s.as_str()).collect();
                let file_test_code =
                    js2rustc::testgen::generate_test_code(&test_cases, &closure_fn_refs);
                all_test_code.push_str(&file_test_code);
            }

            if has_error {
                continue;
            }

            if per_file_modules.is_empty() {
                eprintln!("  skip: no valid modules after codegen");
                continue;
            }

            // --- Generate C ABI wrapper code for lib.zig ---
            // Build name→module lookup and name→CabiExport lookup
            let mut name_to_module: HashMap<&str, &str> = HashMap::new();
            for (exp_name, mod_name) in &all_module_exports {
                name_to_module.entry(exp_name).or_insert(mod_name);
            }
            let mut name_to_cabi: HashMap<&str, &js2rustc::codegen::CabiExport> = HashMap::new();
            for exp in &all_cabi_exports {
                name_to_cabi.entry(&exp.name).or_insert(exp);
            }
            let cabi_wrapper_code = gen_cabi_wrappers(&name_to_module, &name_to_cabi);
            let cabi_names: HashSet<String> =
                name_to_cabi.keys().map(|&k| k.to_string()).collect();

            let project_opts = js2rustc::project::ProjectOptions {
                name: group.core_name.clone(),
                out_dir: out_dir.clone(),
                per_file_code: per_file_modules,
                external_exports: all_module_exports,
                cabi_wrapper_code: cabi_wrapper_code.clone(),
                cabi_names: cabi_names.clone(),
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

            match js2rustc::project::generate(&project_opts) {
                Ok(()) => println!("  Generated: {}/{}", out_dir, group.core_name),
                Err(e) => {
                    eprintln!("  FAIL ({})", e);
                    continue;
                }
            }

            write_cabi_metadata(&out_dir, &group.core_name, &all_cabi_exports, &host_fns, group_idx);
        }

        // === Zig build ===
        let project_path = Path::new(&out_dir).join(&group.core_name);
        let build_result = Command::new("zig")
            .arg("build")
            .current_dir(&project_path)
            .output();
        match build_result {
            Ok(result) if result.status.success() => println!("  zig build: OK"),
            Ok(result) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                eprintln!("  zig build FAILED:\n{}", stderr);
            }
            Err(_) => eprintln!("  warning: zig not found — skipping build"),
        }

        // === Zig tests ===
        let test_result = Command::new("zig")
            .arg("build")
            .arg("test")
            .current_dir(&project_path)
            .output();
        match test_result {
            Ok(result) if result.status.success() => println!("  zig test: PASSED"),
            Ok(result) => {
                let stderr = String::from_utf8_lossy(&result.stderr);
                eprintln!("  zig test FAILED:\n{}", stderr);
            }
            Err(_) => {}
        }
    }
}

/// Generate `pub export fn` wrapper code for lib.zig.
/// Each wrapper calls the per-file module function and lives in the root lib.zig,
/// so Zig correctly propagates the symbols into the final .lib.
///
/// For string-returning functions, ALSO generate a Zig-friendly adapter
/// (`pub fn greet(s: []const u8) []const u8`) so test code can call
/// the function with idiomatic Zig string types.
fn gen_cabi_wrappers(
    name_to_module: &HashMap<&str, &str>,
    name_to_cabi: &HashMap<&str, &js2rustc::codegen::CabiExport>,
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

        let returns_string = exp.ret_type == js2rustc::infer::ZigType::String;

        if returns_string {
            // ── Zig-friendly adapter (for tests) — calls _impl directly, no conversion ──
            out.push_str(&format!(
                "pub fn {name}(s: []const u8) []const u8 {{
    return {mod}.{name}_impl(s);
}}\n",
                name = name,
                mod = module,
            ));

            // ── C ABI wrapper — also calls _impl directly, converting itself ──
            out.push_str(&format!(
                "pub export fn {name}_cabi(name: [*:0]const u8) [*:0]const u8 {{
    const name_slice: []const u8 = std.mem.span(name);
    const _result = {mod}.{name}_impl(name_slice);
    return @ptrCast(_result.ptr);
}}\n",
                name = name,
                mod = module,
            ));
            out.push_str(&format!(
                "comptime {{ @export(&{name}_cabi, .{{ .name = \"{name}\", .linkage = .strong }}); }}\n",
                name = name,
            ));
        } else {
            // ── Simple C ABI wrapper (no string conversion needed) ──
            let mut cabi_params: Vec<String> = Vec::new();
            let mut arg_names: Vec<&str> = Vec::new();
            for (pname, ptype) in &exp.params {
                let zig_ty: String = if *ptype == js2rustc::infer::ZigType::String {
                    "[*:0]const u8".into()
                } else {
                    ptype.to_zig_str()
                };
                cabi_params.push(format!("{}: {}", pname, zig_ty));
                arg_names.push(pname);
            }

            let ret_zig = if exp.ret_type == js2rustc::infer::ZigType::Void {
                "void".to_string()
            } else {
                exp.ret_type.to_zig_str()
            };

            if ret_zig == "void" {
                out.push_str(&format!(
                    "pub export fn {name}({params}) void {{
    {mod}.{name}({args});
}}\n",
                    name = name,
                    params = cabi_params.join(", "),
                    mod = module,
                    args = arg_names.join(", ")
                ));
            } else {
                out.push_str(&format!(
                    "pub export fn {name}({params}) {ret} {{
    return {mod}.{name}({args});
}}\n",
                    name = name,
                    params = cabi_params.join(", "),
                    ret = ret_zig,
                    mod = module,
                    args = arg_names.join(", ")
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
fn write_cabi_metadata(
    out_dir: &str,
    group_name: &str,
    cabi_exports: &[js2rustc::codegen::CabiExport],
    host_fns: &js2rustc::host::HostFnRegistry,
    group_idx: usize,
) {
    let project_dir = Path::new(out_dir).join(group_name);

    // cabi_exports.json
    let exports_path = project_dir.join("cabi_exports.json");
    let mut exports_value: Vec<serde_json::Value> = cabi_exports
        .iter()
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
                "ret_type": exp.ret_type.to_zig_str(),
                "has_free_func": exp.has_free_func
            })
        })
        .collect();

    // Only include js2rust_init and js2rust_deinit for the first group
    // (they're only generated in the first group's lib.zig to avoid duplicate symbols)
    if group_idx == 0 {
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
