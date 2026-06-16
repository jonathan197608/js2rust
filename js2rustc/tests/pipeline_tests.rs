//! Integration tests for the full JS-to-Zig translation pipeline.
//!
//! These tests exercise the end-to-end flow:
//!   preprocess → codegen → Zig project generation → zig build
//!
//! They are integration tests (in `core/tests/`) rather than unit tests
//! because they depend on the `in/` directory and `zig` binary at runtime.

use js2rustc::infer;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Resolve the workspace root directory.
/// In integration tests (`core/tests/`), CARGO_MANIFEST_DIR is `core/`,
/// so the workspace root is the parent.
fn ws_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Full pipeline test: preprocess → codegen → Zig build.
///
/// Runs all 15 JS files in `in/` through the complete translation pipeline
/// and verifies:
/// - Preprocessing produces valid merged JS with no errors
/// - Codegen produces valid Zig with correct export/private visibility
/// - Renaming of internal naming conflicts (suffix strategy)
/// - Zig project builds successfully
#[test]
fn test_full_pipeline() {
    let ws = ws_dir();
    let in_dir = ws.join("in").to_string_lossy().to_string();
    let out_dir = ws.join("out/__rust_test__");
    let out_dir_str = out_dir.to_string_lossy().to_string();
    let project_name = "js2rust_test".to_string();

    // ── Phase 1: Preprocess ──
    let pre_result = js2rustc::preprocess::preprocess(&in_dir);

    let has_error = pre_result.diagnostics.iter().any(|d| d.starts_with("error:"));
    assert!(!has_error, "Preprocessing errors: {:?}", pre_result.diagnostics);
    let merged_js = pre_result.merged_js();
    assert!(
        !merged_js.trim().is_empty(),
        "Merged JS must not be empty"
    );

    // ── Phase 2: Codegen ──
    let exports: HashSet<String> = pre_result.export_map.keys().cloned().collect();
    let allocator = oxc_allocator::Allocator::default();
    let program = js2rustc::parser::parse(&allocator, &merged_js);

    let mut host_fns = js2rustc::host::HostFnRegistry::new();
    host_fns.register(
        "hostAdd",
        vec![
            ("a".into(), infer::ZigType::I64),
            ("b".into(), infer::ZigType::I64),
        ],
        infer::ZigType::I64,
    );
    host_fns.register(
        "hostMultiply",
        vec![
            ("a".into(), infer::ZigType::I64),
            ("b".into(), infer::ZigType::I64),
        ],
        infer::ZigType::I64,
    );

    // Async host function: fetchUser(name) → UserInfo { id, name }
    host_fns.register_async(
        "fetchUser",
        "hostFetchUser",
        vec![("name".into(), infer::ZigType::String)],
        js2rustc::host::HostStructDef {
            zig_name: "UserInfo".into(),
            c_name: "HostUserInfo".into(),
            fields: vec![
                js2rustc::host::HostStructField {
                    name: "id".into(),
                    zig_type: "i64".into(),
                    c_type: "i64".into(),
                },
                js2rustc::host::HostStructField {
                    name: "name".into(),
                    zig_type: "[]const u8".into(),
                    c_type: "[128]u8".into(),
                },
            ],
        },
    );

    let host_header = host_fns.generate_zig_header();
    let async_wrappers = host_fns.generate_async_wrappers();

    let mut builtins = js2rustc::builtins::BuiltinRegistry::new();
    builtins.register_host_fns(&host_fns);
    let (zig_code, diagnostics, _closure_fns, _fn_return_types, _cabi_exports) =
        js2rustc::codegen::generate(&program, &builtins, &exports);

    let has_error = diagnostics
        .iter()
        .any(|d| d.kind == infer::DiagnosticKind::Error);
    assert!(!has_error, "Codegen errors found: {:?}", diagnostics);
    assert!(
        !zig_code.trim().is_empty(),
        "Generated Zig code must not be empty"
    );

    // ── Verify exports ──
    let expected_exports = [
        "export fn chineseAdd(",
        "export fn chineseSub(",
    ];
    for exp in &expected_exports {
        assert!(
            zig_code.contains(exp),
            "Expected export '{exp}' not found in generated Zig code"
        );
    }

    // ── Verify internalized imports (NOT pub) ──
    let internalized = [
        "fn add(",
        "fn multiply(",
        "fn factorial(",
        "fn clamp(",
        "fn bitAnd(",
        "fn bitOr(",
        "fn bitXor(",
        "fn bitNot(",
        "fn bitShift(",
        "fn voidFunc(",
        "fn fetchData(",
        "fn processItems(",
        "fn makeAdder(",
        "fn greet(",
    ];
    for name in &internalized {
        assert!(
            zig_code.contains(name),
            "Internalized fn '{name}' should be present in generated Zig code"
        );
        let with_pub = format!("pub {name}");
        assert!(
            !zig_code.contains(&with_pub),
            "Internalized fn '{name}' should NOT be pub"
        );
    }

    // ── Verify non-exported internal functions ──
    let internal_names = [
        "fn getData_async_ops",
        "fn saveResults_async_ops",
        "fn helper_utils",
        "fn helper_strings",
    ];
    for name in &internal_names {
        assert!(
            zig_code.contains(name),
            "Internal fn '{name}' should be present in generated Zig code"
        );
        let with_pub = format!("pub {name}");
        assert!(
            !zig_code.contains(&with_pub),
            "Internal function '{name}' should not be pub"
        );
    }

    // ── Phase 3: Generate project + zig build ──
    fs::create_dir_all(&out_dir).unwrap();
    let project_opts = js2rustc::project::ProjectOptions {
        name: project_name,
        out_dir: out_dir_str.clone(),
        lib_code: zig_code,
        per_file_code: vec![],
        external_exports: vec![],
        test_code: String::new(),
        runtime_dir: Some(ws.join("runtime").to_string_lossy().to_string()),
        host_header,
        async_host_wrappers: async_wrappers,
    };
    js2rustc::project::generate(&project_opts).expect("Project generation must succeed");

    let project_path = out_dir.join("js2rust_test");
    assert!(project_path.exists(), "Project directory must exist");

    let build_result = Command::new("zig")
        .arg("build")
        .current_dir(&project_path)
        .output();

    match build_result {
        Ok(result) => {
            assert!(
                result.status.success(),
                "zig build failed:\n{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        Err(e) => {
            eprintln!("warning: zig not available for testing: {e}");
        }
    }

    // Cleanup
    let _ = fs::remove_dir_all(&out_dir);
}

/// Internal naming conflicts are resolved via suffix renaming
/// (e.g. `helper()` in utils.js → `helper_utils`, in strings.js → `helper_strings`).
#[test]
fn test_suffix_renaming() {
    let in_dir = ws_dir().join("in").to_string_lossy().to_string();
    let pre = js2rustc::preprocess::preprocess(&in_dir);
    let merged_js = pre.merged_js();

    assert!(
        merged_js.contains("helper_utils"),
        "Expected 'helper_utils' in merged JS (internal rename)"
    );
    assert!(
        merged_js.contains("helper_strings"),
        "Expected 'helper_strings' in merged JS (internal rename)"
    );
}

/// Export naming conflicts are detected (no false positives).
#[test]
fn test_no_export_conflicts() {
    let in_dir = ws_dir().join("in").to_string_lossy().to_string();
    let pre = js2rustc::preprocess::preprocess(&in_dir);

    let has_export_error = pre.diagnostics
        .iter()
        .any(|d| d.starts_with("error:") && d.contains("export"));
    assert!(!has_export_error, "Should have no export naming conflicts");
}

/// All JS modules are processed and merged.
#[test]
fn test_all_modules_processed() {
    let in_dir = ws_dir().join("in").to_string_lossy().to_string();
    let pre = js2rustc::preprocess::preprocess(&in_dir);
    let merged_js = pre.merged_js();

    let expected_markers = [
        "from math.js",
        "from utils.js",
        "from strings.js",
        "from async_ops.js",
        "from bitwise_ops.js",
        "from main.js",
    ];
    for marker in &expected_markers {
        assert!(
            merged_js.contains(marker),
            "Expected marker '{marker}' not found in merged JS"
        );
    }
}
