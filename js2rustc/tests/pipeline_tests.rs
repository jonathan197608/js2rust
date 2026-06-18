//! Integration tests for the JS-to-Zig translation pipeline.
//!
//! These tests exercise the end-to-end flow:
//!   analyze_groups → strip imports → codegen → Zig project → zig build
//!
//! They are integration tests (in `core/tests/`) rather than unit tests
//! because they depend on the `in/` directory and `zig` binary at runtime.

use js2rustc::analyzer::{analyze_groups, strip_imports_extract_exports};
use js2rustc::infer;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve the workspace root directory.
fn ws_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Create a temporary directory with given JS files, returning the path.
fn setup_temp_in(files: &[(&str, &str)]) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!("js2rust_pipeline_test_{id}"));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    for (name, content) in files {
        fs::write(tmp.join(name), content).unwrap();
    }
    tmp
}

// ═══════════════════════════════════════════════════════════════════
//  Unit tests for strip_imports_extract_exports
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_strip_imports_basic() {
    let src = r#"import { foo } from "./bar";
function test() { return 1; }"#;
    let (cleaned, exports) = strip_imports_extract_exports(src);
    assert!(!cleaned.contains("import"), "import should be stripped");
    assert!(cleaned.contains("function test"), "function should remain");
    assert!(exports.is_empty(), "no exports in this file");
}

#[test]
fn test_extract_exports() {
    let src = r#"export function add(a,b) { return a+b; }
export const PI = 3.14;
export class Point { x; y; }
function helper() { return 0; }"#;
    let (cleaned, exports) = strip_imports_extract_exports(src);
    assert!(!cleaned.contains("export "), "export keyword should be stripped from declarations");
    assert!(cleaned.contains("function add"), "function body should remain");
    assert!(cleaned.contains("const PI"), "const should remain");
    assert!(cleaned.contains("class Point"), "class should remain");
    assert!(cleaned.contains("function helper"), "internal fn should remain");
    assert!(exports.contains("add"), "add should be detected");
    assert!(exports.contains("PI"), "PI should be detected");
    assert!(exports.contains("Point"), "Point should be detected");
    assert!(!exports.contains("helper"), "helper should NOT be exported");
}

#[test]
fn test_strip_export_default() {
    let src = "export default function foo() { return 1; }";
    let (cleaned, _) = strip_imports_extract_exports(src);
    assert!(!cleaned.contains("export "));
    assert!(cleaned.contains("function foo"));
}

#[test]
fn test_strip_empty_src() {
    let (cleaned, exports) = strip_imports_extract_exports("");
    assert!(cleaned.trim().is_empty());
    assert!(exports.is_empty());
}

// ═══════════════════════════════════════════════════════════════════
//  Analyzer tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_analyzer_single_file() {
    let tmp = setup_temp_in(&[("main.js", "function add(a,b) { return a+b; }")]);
    let in_dir = tmp.to_string_lossy().to_string();
    let (groups, _) = analyze_groups(&in_dir);
    assert_eq!(groups.len(), 1, "single file → one group");
    assert_eq!(groups[0].core_file, "main.js");
    assert_eq!(groups[0].members.len(), 1);
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_analyzer_with_imports() {
    let tmp = setup_temp_in(&[
        ("math.js", "export function add(a,b) { return a+b; }"),
        ("main.js", "import { add } from './math.js';\nfunction test() { return add(1,2); }"),
    ]);
    let in_dir = tmp.to_string_lossy().to_string();
    let (groups, _) = analyze_groups(&in_dir);
    assert_eq!(groups.len(), 1, "main.js is core, math.js is a dependency");
    assert_eq!(groups[0].core_file, "main.js");
    assert_eq!(groups[0].members.len(), 2);
    // math.js should come before main.js (topological order)
    assert_eq!(groups[0].members[0], "math.js");
    assert_eq!(groups[0].members[1], "main.js");
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_analyzer_two_cores() {
    let tmp = setup_temp_in(&[
        ("app1.js", "function run1() { return 1; }"),
        ("app2.js", "function run2() { return 2; }"),
    ]);
    let in_dir = tmp.to_string_lossy().to_string();
    let (groups, _) = analyze_groups(&in_dir);
    assert_eq!(groups.len(), 2, "two independent files → two groups");
    let _ = fs::remove_dir_all(&tmp);
}

// ═══════════════════════════════════════════════════════════════════
//  Full pipeline test (multi-file mode: lib.zig + xxx.zig)
// ═══════════════════════════════════════════════════════════════════

/// Full pipeline: analyzer → per-file codegen → project → zig build.
#[test]
fn test_full_pipeline() {
    let tmp = setup_temp_in(&[(
        "main.js",
        r#"
export function add(a, b) {
    return a + b;
}

export function multiply(a, b) {
    return a * b;
}

function helper(x) {
    return x + 1;
}

function factorial(n) {
    if (n <= 1) { return 1; }
    return n * factorial(n - 1);
}

const test_add = add(3, 5); // => 8
const test_mult = multiply(4, 7); // => 28
const test_fact = factorial(5); // => 120
"#,
    )]);
    let in_dir = tmp.to_string_lossy().to_string();

    // Phase 1: Analyze
    let (groups, _) = analyze_groups(&in_dir);
    assert_eq!(groups.len(), 1);
    let group = &groups[0];

    // Phase 2: Per-file codegen (single file → single PerFileModule)
    let member = &group.members[0];
    let src = fs::read_to_string(tmp.join(member)).unwrap();
    let (stripped, exports) = strip_imports_extract_exports(&src);

    let mut host_fns = js2rustc::host::HostFnRegistry::new();
    host_fns.register(
        "hostAdd",
        vec![
            ("a".into(), infer::ZigType::I64),
            ("b".into(), infer::ZigType::I64),
        ],
        infer::ZigType::I64,
    );

    let mut builtins = js2rustc::builtins::BuiltinRegistry::new();
    builtins.register_host_fns(&host_fns);
    let host_header = host_fns.generate_zig_header();
    let async_wrappers = host_fns.generate_async_wrappers();

    let allocator = oxc_allocator::Allocator::default();
    let program = js2rustc::parser::parse(&allocator, &stripped);
    let (zig_code, diagnostics, closure_fns, _, _, _) =
        js2rustc::codegen::generate(&program, &builtins, &exports, &stripped, "test.js");

    let has_error = diagnostics
        .iter()
        .any(|d| d.kind == infer::DiagnosticKind::Error);
    assert!(!has_error, "Codegen errors: {:?}", diagnostics);
    assert!(!zig_code.trim().is_empty(), "Zig code must not be empty");

    // In multi-file mode, exports in per-file modules use `pub fn`
    // (C ABI `pub export fn` wrappers are generated in the orchestrator lib.zig)
    assert!(
        zig_code.contains("pub fn add("),
        "add should be pub fn"
    );
    assert!(
        zig_code.contains("pub fn multiply("),
        "multiply should be pub fn"
    );
    assert!(
        zig_code.contains("pub fn helper("),
        "helper should be pub fn"
    );
    assert!(
        zig_code.contains("pub fn factorial("),
        "factorial should be pub fn"
    );

    // Collect all pub functions for orchestrator re-export
    let mut all_exports: Vec<(String, String)> = exports
        .iter()
        .map(|e| (e.clone(), "main".into()))
        .collect();
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
            if name != "init_js2rust"
                && name != "deinit_js2rust"
                && !exports.contains(&name)
            {
                all_exports.push((name, "main".into()));
            }
        }
    }

    // Testgen
    let test_cases = js2rustc::testgen::extract_test_cases(&program, &stripped);
    let clos_refs: HashSet<&str> = closure_fns.iter().map(|s| s.as_str()).collect();
    let test_code = js2rustc::testgen::generate_test_code(&test_cases, &clos_refs, &HashMap::new());

    // Phase 3: Project gen (multi-file mode: lib.zig + main.zig)
    let out_dir = std::env::temp_dir().join("js2rust_test_out");
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).unwrap();
    let out_dir_str = out_dir.to_string_lossy().to_string();

    let project_opts = js2rustc::project::ProjectOptions {
        name: group.core_name.clone(),
        out_dir: out_dir_str,
        per_file_code: vec![js2rustc::project::PerFileModule {
            mod_name: "main".into(),
            zig_code: zig_code.clone(),
            dep_imports: vec![],
        }],
        external_exports: all_exports,
        cabi_wrapper_code: String::new(),
        cabi_names: Default::default(),
        test_code,
        runtime_dir: Some(ws_dir().join("runtime").to_string_lossy().to_string()),
        host_header,
        async_host_wrappers: async_wrappers,
        include_windows_stub: false,
    };
    js2rustc::project::generate(&project_opts).expect("Project gen must succeed");

    let project_path = out_dir.join(&group.core_name);
    assert!(project_path.exists(), "Project directory must exist");
    assert!(
        project_path.join("src/lib.zig").exists(),
        "lib.zig must exist"
    );
    assert!(
        project_path.join("src/main.zig").exists(),
        "main.zig must exist"
    );

    match Command::new("zig")
        .arg("build")
        .current_dir(&project_path)
        .output()
    {
        Ok(result) => {
            assert!(
                result.status.success(),
                "zig build failed:\n{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        Err(e) => {
            eprintln!("warning: zig not available: {e}");
        }
    }

    // Run zig tests
    if let Ok(result) = Command::new("zig")
        .arg("build")
        .arg("test")
        .current_dir(&project_path)
        .output()
    {
        assert!(
            result.status.success(),
            "zig test failed:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    let _ = fs::remove_dir_all(&tmp);
    let _ = fs::remove_dir_all(&out_dir);
}
