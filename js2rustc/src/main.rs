use js2rustc::analyzer::analyze_groups;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve paths relative to the workspace root (parent of core crate).
fn workspace_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn main() {
    let ws = workspace_dir();
    let in_dir = ws.join("in").to_string_lossy().to_string();
    let out_dir = ws.join("out").to_string_lossy().to_string();
    let project_name = "js2rust".to_string();

    // Ensure output directory exists before any writes.
    fs::create_dir_all(&out_dir).unwrap_or_else(|e| {
        eprintln!("error: cannot create output directory '{}': {}", out_dir, e);
        std::process::exit(1);
    });

    // === Phase 1: Preprocess JS files (merge + resolve imports/exports) ===
    let pre_result = js2rustc::preprocess::preprocess(&in_dir);

    for diag in &pre_result.diagnostics {
        eprintln!("{}", diag);
    }
    let has_error = pre_result.diagnostics.iter().any(|d| d.starts_with("error:"));
    if has_error {
        eprintln!("Preprocessing failed -- aborting.");
        std::process::exit(1);
    }

    let merged_js = pre_result.merged_js();
    if merged_js.trim().is_empty() {
        eprintln!("error: no JS source after preprocessing");
        std::process::exit(1);
    }

    // === Phase 1.5: Analyze file groups for multi-file Zig projects ===
    let (groups, groups_json) = analyze_groups(&in_dir);
    let groups_json_path = Path::new(&out_dir).join("groups.json");
    if let Err(e) = fs::write(&groups_json_path, &groups_json) {
        eprintln!("warning: could not write '{}': {}", groups_json_path.display(), e);
    } else {
        println!("Wrote: {}/groups.json", out_dir);
    }

    // Build export set: function names that should be `pub fn` in Zig
    let exports: HashSet<String> = pre_result.export_map.keys().cloned().collect();

    // === Phase 2: Type inference + codegen on merged JS ===
    let allocator = oxc_allocator::Allocator::default();
    let program = js2rustc::parser::parse(&allocator, &merged_js);

    // === Register host functions (Rust functions callable from JS) ===
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
    let (zig_code, diagnostics, closure_fns, _fn_return_types, _cabi_exports) =
        js2rustc::codegen::generate(&program, &builtins, &exports);

    // Check for errors
    let has_error = diagnostics
        .iter()
        .any(|d| d.kind == js2rustc::infer::DiagnosticKind::Error);
    if has_error {
        eprintln!(
            "\nMerged JS: {} error(s) -- skipping project generation",
            diagnostics
                .iter()
                .filter(|d| d.kind == js2rustc::infer::DiagnosticKind::Error)
                .count()
        );
        for diag in &diagnostics {
            match diag.kind {
                js2rustc::infer::DiagnosticKind::Error => {
                    eprintln!("  {}", diag.format_with_source(&merged_js))
                }
                js2rustc::infer::DiagnosticKind::Warning => {
                    eprintln!("  {}", diag.format_with_source(&merged_js))
                }
            }
        }
        std::process::exit(1);
    }

    // Print diagnostics
    if !diagnostics.is_empty() {
        eprintln!("{}: {} diagnostic(s)", project_name, diagnostics.len());
        for diag in &diagnostics {
            eprintln!("  {}", diag.format_with_source(&merged_js));
        }
    }

    // === Write C ABI exports metadata to out/cabi_exports.json ===
    // This file is read by sys/build.rs to generate ffi_bindings.rs
    // without depending on the core crate.
    let cabi_json_path = Path::new(&out_dir).join("cabi_exports.json");
    let cabi_json_value: Vec<serde_json::Value> = _cabi_exports
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
    match serde_json::to_string_pretty(&cabi_json_value) {
        Ok(json_str) => {
            if let Err(e) = fs::write(&cabi_json_path, &json_str) {
                eprintln!(
                    "warning: could not write '{}': {}",
                    cabi_json_path.display(),
                    e
                );
            } else {
                println!("Wrote: {}/cabi_exports.json", out_dir);
            }
        }
        Err(e) => {
            eprintln!("warning: serializing cabi_exports: {}", e);
        }
    }

    // === Write C ABI imports metadata to out/cabi_imports.json ===
    // This file is read by sys/build.rs to link Rust host functions.
    let cabi_imports_path = Path::new(&out_dir).join("cabi_imports.json");
    let cabi_imports_value = host_fns.to_json_value();
    match serde_json::to_string_pretty(&cabi_imports_value) {
        Ok(json_str) => {
            if let Err(e) = fs::write(&cabi_imports_path, &json_str) {
                eprintln!(
                    "warning: could not write '{}': {}",
                    cabi_imports_path.display(),
                    e
                );
            } else {
                println!("Wrote: {}/cabi_imports.json", out_dir);
            }
        }
        Err(e) => {
            eprintln!("warning: serializing cabi_imports: {}", e);
        }
    }

    // === Phase 3: Extract test cases from AST (smoke tests, no JS runtime) ===
    let test_cases = js2rustc::testgen::extract_test_cases(&program);
    let closure_fn_refs: HashSet<&str> = closure_fns.iter().map(|s| s.as_str()).collect();
    let test_code = js2rustc::testgen::generate_test_code(&test_cases, &closure_fn_refs);

    // === Phase 4: Generate Zig project ===
    let project_opts = js2rustc::project::ProjectOptions {
        name: project_name.clone(),
        out_dir: out_dir.clone(),
        lib_code: zig_code,
        per_file_code: Vec::new(),
        external_exports: Vec::new(),
        test_code,
        runtime_dir: Some(ws.join("runtime").to_string_lossy().to_string()),
        host_header: host_fns.generate_zig_header(),
        async_host_wrappers: host_fns.generate_async_wrappers(),
    };

    if let Err(e) = js2rustc::project::generate(&project_opts) {
        eprintln!("error: generating project '{}': {}", project_name, e);
        std::process::exit(1);
    }

    println!("Generated: {}/{} (single Zig library)", out_dir, project_name);

    // === Phase 5: Build + test the generated Zig project ===
    let project_path = Path::new(&out_dir).join(&project_name);
    let output = Command::new("zig")
        .arg("build")
        .current_dir(&project_path)
        .output()
        .ok();

    if let Some(result) = output {
        if result.status.success() {
            println!("Zig build: OK");
        } else {
            let stderr = String::from_utf8_lossy(&result.stderr);
            eprintln!("Zig build failed:\n{}", stderr);
        }
    } else {
        eprintln!("warning: zig not found -- skipping build");
    }

    // === Phase 6: Run Zig tests ===
    let test_output = Command::new("zig")
        .arg("build")
        .arg("test")
        .current_dir(&project_path)
        .output()
        .ok();

    if let Some(result) = test_output {
        if result.status.success() {
            println!("Zig tests: PASSED");
        } else {
            let stderr = String::from_utf8_lossy(&result.stderr);
            eprintln!("Zig tests FAILED:\n{}", stderr);
        }
    }

    // === Phase 7: Generate one Zig project per file group ===
    if groups.len() > 1 {
        println!(
            "\n=== Generating {} file-group Zig projects ===",
            groups.len()
        );

        let host_header = host_fns.generate_zig_header();
        let async_wrappers = host_fns.generate_async_wrappers();
        let runtime_dir = ws.join("runtime").to_string_lossy().to_string();

        for group in &groups {
            // Create temp in-dir with only this group's files
            let tmp_in = format!("{}/_tmp_group_{}", out_dir, group.core_name);
            let _ = fs::remove_dir_all(&tmp_in);
            fs::create_dir_all(&tmp_in).unwrap_or_else(|e| {
                eprintln!("warning: cannot create '{}': {}", tmp_in, e);
            });

            for member in &group.members {
                let src = ws.join("in").join(member);
                let dst = Path::new(&tmp_in).join(member);
                let _ = fs::copy(&src, &dst);
            }

            // Preprocess for this group
            let merge = js2rustc::preprocess::preprocess(&tmp_in);
            let has_err = merge.diagnostics.iter().any(|d| d.starts_with("error:"));
            if has_err || merge.merged_js().trim().is_empty() {
                eprintln!(
                    "  skip '{}': preprocessing errors ({})",
                    group.core_name,
                    merge.diagnostics.len()
                );
                let _ = fs::remove_dir_all(&tmp_in);
                continue;
            }

            let is_multi = group.members.len() > 1;

            if is_multi {
                // === Multi-member group: per-file codegen ===
                let mut per_file_zig: Vec<(String, String)> = Vec::new();
                let mut external_exports: Vec<(String, String)> = Vec::new();
                let mut all_closures: Vec<String> = Vec::new();
                let mut has_host = false;
                let mut has_async = false;

                // Build all-exports map (all exports from ALL files → source module)
                // for lib.zig re-exports so test blocks can reference functions directly.
                let mut all_exports_map: HashMap<String, String> = HashMap::new();

                for (mod_name, transformed_js) in &merge.per_file {
                    if transformed_js.trim().is_empty() {
                        continue;
                    }

                    let file_exports: HashSet<String> = merge
                        .per_file_exports
                        .get(mod_name)
                        .cloned()
                        .unwrap_or_default();

                    // Track all exports → source module for lib.zig re-exports
                    for exp in &file_exports {
                        all_exports_map.insert(exp.clone(), mod_name.clone());
                    }

                    let alloc = oxc_allocator::Allocator::default();
                    let prog = js2rustc::parser::parse(&alloc, transformed_js);

                    // Per-file codegen with EMPTY exports: produces `fn` (not `export fn`).
                    // We post-process to add `pub` for exported functions.
                    let empty_exports: HashSet<String> = HashSet::new();
                    let (zig, diag, clos, _ret_types, _) = js2rustc::codegen::generate(
                        &prog,
                        &builtins,
                        &empty_exports,
                    );

                    if diag
                        .iter()
                        .any(|d| d.kind == js2rustc::infer::DiagnosticKind::Error)
                    {
                        eprintln!("  skip '{}/{}': codegen errors", group.core_name, mod_name);
                        continue;
                    }

                    // Post-process: add `pub` prefix to exported functions.
                    // The codegen produces `fn add(` — we change it to `pub fn add(`.
                    let zig_pub = add_pub_to_exports(&zig, &file_exports);
                    // Strip per-file init_js2rust/deinit_js2rust (provided by orchestrator lib.zig)
                    let zig_clean = strip_init_deinit(&zig_pub);

                    // Track closure structs for testgen
                    all_closures.extend(clos);

                    // Check for host/async references
                    if !has_host && zig_clean.contains("host.") {
                        has_host = true;
                    }
                    if !has_async && zig_clean.contains("fetchUser") {
                        has_async = true;
                    }

                    per_file_zig.push((mod_name.clone(), zig_clean));
                }

                if per_file_zig.is_empty() {
                    eprintln!("  skip '{}': no valid per-file codegen results", group.core_name);
                    let _ = fs::remove_dir_all(&tmp_in);
                    continue;
                }

                // Re-export ALL exports in lib.zig so test blocks can reference
                // functions directly (e.g., `add(3,5)` not `math.add(3,5)`).
                for (exp_name, mod_name) in &all_exports_map {
                    external_exports.push((exp_name.clone(), mod_name.clone()));
                }

                // Run testgen on merged program AST
                let alloc_m = oxc_allocator::Allocator::default();
                let merged = merge.merged_js();
                let prog_m = js2rustc::parser::parse(&alloc_m, &merged);
                let test_cases = js2rustc::testgen::extract_test_cases(&prog_m);
                let clos_refs: HashSet<&str> =
                    all_closures.iter().map(|s| s.as_str()).collect();
                let test_code = js2rustc::testgen::generate_test_code(&test_cases, &clos_refs);

                let proj_opts = js2rustc::project::ProjectOptions {
                    name: group.core_name.clone(),
                    out_dir: format!("{}/groups", out_dir),
                    lib_code: String::new(), // not used in multi-file mode
                    per_file_code: per_file_zig,
                    external_exports,
                    test_code,
                    runtime_dir: Some(runtime_dir.clone()),
                    host_header: if has_host { host_header.clone() } else { String::new() },
                    async_host_wrappers: if has_async { async_wrappers.clone() } else { String::new() },
                };

                match js2rustc::project::generate(&proj_opts) {
                    Ok(()) => println!(
                        "  {}/: OK ({} files → {} modules)",
                        group.core_name,
                        group.members.len(),
                        proj_opts.per_file_code.len()
                    ),
                    Err(e) => eprintln!("  {}/: FAIL ({})", group.core_name, e),
                }
            } else {
                // === Single-member group: merged codegen (existing path) ===
                let group_exports: HashSet<String> =
                    merge.export_map.keys().cloned().collect();
                let alloc2 = oxc_allocator::Allocator::default();
                let merged2 = merge.merged_js();
                let prog2 = js2rustc::parser::parse(&alloc2, &merged2);

                let (zig2, diag2, clos2, _ret_types2, _) = js2rustc::codegen::generate(
                    &prog2,
                    &builtins,
                    &group_exports,
                );

                if diag2
                    .iter()
                    .any(|d| d.kind == js2rustc::infer::DiagnosticKind::Error)
                {
                    eprintln!("  skip '{}': codegen errors", group.core_name);
                    let _ = fs::remove_dir_all(&tmp_in);
                    continue;
                }

                let test_cases2 = js2rustc::testgen::extract_test_cases(&prog2);
                let clos_refs: HashSet<&str> =
                    clos2.iter().map(|s| s.as_str()).collect();
                let test_code2 =
                    js2rustc::testgen::generate_test_code(&test_cases2, &clos_refs);

                let proj_opts = js2rustc::project::ProjectOptions {
                    name: group.core_name.clone(),
                    out_dir: format!("{}/groups", out_dir),
                    lib_code: zig2.clone(),
                    per_file_code: Vec::new(),
                    external_exports: Vec::new(),
                    test_code: test_code2,
                    runtime_dir: Some(runtime_dir.clone()),
                    host_header: if zig2.contains("host.") { host_header.clone() } else { String::new() },
                    async_host_wrappers: if zig2.contains("fetchUser") { async_wrappers.clone() } else { String::new() },
                };

                match js2rustc::project::generate(&proj_opts) {
                    Ok(()) => println!(
                        "  {}/: OK ({} members)",
                        group.core_name,
                        group.members.len()
                    ),
                    Err(e) => eprintln!("  {}/: FAIL ({})", group.core_name, e),
                }
            }

            let _ = fs::remove_dir_all(&tmp_in);
        }
    }
}

/// Post-process per-file Zig output: add `pub` prefix to functions
/// that are in the exports set (from JS `export function`).
/// The codegen passes empty exports set so functions are `fn`, not `export fn`.
fn add_pub_to_exports(zig_code: &str, exports: &HashSet<String>) -> String {
    if exports.is_empty() {
        return zig_code.to_string();
    }

    let mut result = String::with_capacity(zig_code.len() + exports.len() * 4);
    let bytes = zig_code.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Look for "fn " or "pub fn " (already public)
        if bytes[i..].starts_with(b"pub fn ") {
            // Already public — skip
            let end = i + 7;
            result.push_str(&zig_code[i..end]);
            i = end;
            continue;
        }

        if bytes[i..].starts_with(b"fn ") {
            // Check if this function name is in the exports set
            let fn_start = i;
            let name_start = i + 3; // after "fn "
            let name_end = match bytes[name_start..].iter().position(|&b| b == b'(' || b == b'<') {
                Some(pos) => name_start + pos,
                None => {
                    // Not a function definition — just copy
                    result.push_str(&zig_code[i..i + 2]);
                    i += 2;
                    continue;
                }
            };

            let fn_name = &zig_code[name_start..name_end];

            // Check against exports (exact match)
            if exports.contains(fn_name) {
                result.push_str("pub fn ");
                i = fn_start + 3; // skip "fn "
            } else {
                result.push_str("fn ");
                i = fn_start + 3;
            }
            continue;
        }

        // Copy one byte
        result.push(zig_code[i..].chars().next().unwrap());
        i += zig_code[i..].chars().next().unwrap().len_utf8();
    }

    result
}

/// Strip init_js2rust and deinit_js2rust from per-file module code.
/// These are already provided by the orchestrator lib.zig.
fn strip_init_deinit(zig_code: &str) -> String {
    // Remove lines containing "pub fn init_js2rust" / "pub fn deinit_js2rust"
    // and their associated comment lines and bodies
    let mut result = String::with_capacity(zig_code.len());
    let mut in_init_block = false;
    let mut in_deinit_block = false;
    let mut brace_depth: i32 = 0;

    for line in zig_code.lines() {
        let trimmed = line.trim();

        // Detect start of init_js2rust or deinit_js2rust definitions and doc comments
        if trimmed == "/// Initialize global allocator and all objects that use dynamic property access."
            || trimmed == "/// Release global resources allocated via init_js2rust()."
            || trimmed == "/// Initialize the global allocator used by all generated functions."
            || trimmed == "/// Deinitialize all global objects created by init_js2rust()."
        {
            // Check next line to confirm it's about init_js2rust/deinit_js2rust
            in_init_block = false;
            in_deinit_block = false;
            continue; // skip the doc comment
        }

        if trimmed.starts_with("pub fn init_js2rust(") {
            in_init_block = true;
            brace_depth = 0;
            // Count braces on this line
            for ch in line.chars() {
                if ch == '{' { brace_depth += 1; }
                if ch == '}' { brace_depth -= 1; }
            }
            if brace_depth <= 0 {
                in_init_block = false;
            }
            continue;
        }

        if trimmed.starts_with("pub fn deinit_js2rust(") {
            in_deinit_block = true;
            brace_depth = 0;
            for ch in line.chars() {
                if ch == '{' { brace_depth += 1; }
                if ch == '}' { brace_depth -= 1; }
            }
            if brace_depth <= 0 {
                in_deinit_block = false;
            }
            continue;
        }

        if in_init_block || in_deinit_block {
            for ch in line.chars() {
                if ch == '{' { brace_depth += 1; }
                if ch == '}' { brace_depth -= 1; }
            }
            if brace_depth <= 0 {
                in_init_block = false;
                in_deinit_block = false;
            }
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}
