// Shared test infrastructure for js2zig-core.

use crate::native_proto::transpile_js;
use crate::types::TranspileResult;
use std::collections::HashSet;

/// Helper: parse JS source, then call transpile_js with the parsed Program.
/// Wraps the two-arg API for test convenience.
pub fn parse_and_transpile(
    js: &str,
    exports: Option<HashSet<String>>,
) -> Result<TranspileResult, String> {
    let alloc = oxc_allocator::Allocator::default();
    let program = crate::parser::parse(&alloc, js);
    transpile_js(&program, js, exports, None, "test")
}

/// Helper: run `zig ast-check` on generated Zig code.
/// Panics if ast-check fails (to fail the test).
/// Skips gracefully if `zig` is not installed.
/// Automatically adds `const std = @import("std");` and `const allocator = ...`
/// if the generated code references `std.` or `allocator` (self-contained for ast-check).
pub fn assert_zig_ast_check(zig_code: &str, test_name: &str) {
    // Check which runtime imports are needed.
    let needs_std = zig_code.contains("std.") || zig_code.contains("allocator");
    let needs_js_date = zig_code.contains("js_date");
    let needs_js_object = zig_code.contains("js_object");
    let needs_js_number = zig_code.contains("js_number.");
    let needs_js_string = zig_code.contains("js_string.");
    let needs_js_runtime = zig_code.contains("js_runtime.");
    let needs_js_any = zig_code.contains("JsAny");
    let needs_string_hashmap = zig_code.contains("StringHashMap");
    let needs_js_allocator = zig_code.contains("js_allocator");
    let needs_js_array = zig_code.contains("js_array");
    let needs_js_json = zig_code.contains("js_json");
    let needs_js_collections = zig_code.contains("js_collections");
    let needs_js_uri = zig_code.contains("js_uri.");
    let needs_js_regexp = zig_code.contains("js_regexp.");
    let needs_js_symbol = zig_code.contains("js_symbol.") || zig_code.contains("JsSymbol");
    let needs_js_bigint = zig_code.contains("js_bigint.");
    let needs_js_error = zig_code.contains("js_error.");
    let needs_js_string_icu = zig_code.contains("js_string_icu");
    let needs_js_string_regex = zig_code.contains("js_string_regex");
    let any_runtime = needs_js_date
        || needs_js_object
        || needs_js_number
        || needs_js_string
        || needs_js_runtime
        || needs_js_any
        || needs_string_hashmap
        || needs_js_allocator
        || needs_js_array
        || needs_js_collections
        || needs_js_uri
        || needs_js_regexp
        || needs_js_symbol
        || needs_js_bigint
        || needs_js_error
        || needs_js_string_icu
        || needs_js_string_regex;

    let wrapped = if needs_std || any_runtime {
        let mut w = String::new();
        w.push_str("const std = @import(\"std\");\n");
        w.push_str("const allocator = std.heap.page_allocator;\n");
        if needs_js_allocator {
            w.push_str("const js_allocator = @import(\"js_runtime/js_allocator.zig\");\n");
        }
        if needs_js_array {
            w.push_str("const js_array = @import(\"js_runtime/js_array.zig\");\n");
        }
        if needs_js_json {
            w.push_str("const js_json = @import(\"js_runtime/js_json.zig\");\n");
        }
        if needs_js_collections {
            w.push_str("const js_collections = @import(\"js_runtime/js_collections.zig\");\n");
        }
        if needs_js_uri {
            w.push_str("const js_uri = @import(\"js_runtime/js_uri.zig\");\n");
        }
        if needs_js_regexp {
            w.push_str("const js_regexp = @import(\"js_runtime/js_regexp.zig\");\n");
        }
        if needs_js_date {
            w.push_str("const js_date = @import(\"js_runtime/js_date.zig\");\n");
        }
        if needs_js_object {
            w.push_str("const js_object = @import(\"js_runtime/js_object.zig\");\n");
        }
        if needs_js_number {
            w.push_str("const js_number = @import(\"js_runtime/js_number.zig\");\n");
        }
        if needs_js_string {
            w.push_str("const js_string = @import(\"js_runtime/js_string.zig\");\n");
        }
        if needs_js_runtime {
            w.push_str("const js_runtime = @import(\"js_runtime/js_runtime.zig\");\n");
        }
        if needs_js_any {
            w.push_str("const JsAny = @import(\"js_runtime/jsany.zig\").JsAny;\n");
        }
        if needs_js_symbol {
            w.push_str("const js_symbol = @import(\"js_runtime/js_symbol.zig\");\n");
            // Also import the JsSymbol type for function signatures
            w.push_str("const JsSymbol = @import(\"js_runtime/js_symbol.zig\").JsSymbol;\n");
        }
        if needs_js_bigint {
            w.push_str("const js_bigint = @import(\"js_runtime/js_bigint.zig\");\n");
        }
        if needs_js_error {
            w.push_str("const js_error = @import(\"js_runtime/js_error.zig\");\n");
        }
        if needs_js_string_icu {
            w.push_str("const js_string_icu = @import(\"js_runtime/js_string_icu.zig\");\n");
        }
        if needs_js_string_regex {
            w.push_str("const js_string_regex = @import(\"js_runtime/js_string_regex.zig\");\n");
        }
        if needs_string_hashmap {
            w.push_str("const StringHashMap = std.StringHashMap;\n");
        }
        w.push('\n');
        w.push_str(zig_code);
        w
    } else {
        zig_code.to_string()
    };

    let tmp_dir = std::env::temp_dir();
    let zig_path = tmp_dir.join(format!("{}.zig", test_name));
    let wrapped_ref: &str = &wrapped;
    std::fs::write(&zig_path, wrapped_ref).unwrap();

    match std::process::Command::new("zig.exe")
        .args(["ast-check", zig_path.to_str().unwrap()])
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("=== zig ast-check failed for {} ===", test_name);
                eprintln!("Generated code:\n{}", wrapped);
                eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
                panic!("zig ast-check failed");
            } else {
                println!("=== zig ast-check passed for {} ===", test_name);
            }
        }
        Err(e) => {
            eprintln!("Failed to run zig ast-check (skipping): {}", e);
        }
    }
}

/// Transpile JS, print generated Zig, return Zig code (no ast-check).
/// Replaces the `transpile_and_assert!` macro.
pub fn transpile_and_assert(js: &str, test_name: &str) -> String {
    let result = parse_and_transpile(js, None).unwrap();
    println!("=== Generated Zig ({}) ===\n{}", test_name, result.zig_code);
    result.zig_code
}

/// Transpile JS, print, run ast-check, return Zig code.
/// For the overload with `exports`, see [`transpile_and_check_with_exports`].
/// Replaces the 2-arg `transpile_and_check!` macro.
pub fn transpile_and_check(js: &str, test_name: &str) -> String {
    let result = parse_and_transpile(js, None).unwrap();
    println!("=== Generated Zig ({}) ===\n{}", test_name, result.zig_code);
    assert_zig_ast_check(&result.zig_code, test_name);
    result.zig_code
}

/// Transpile JS with explicit exports, print, run ast-check, return Zig code.
/// Replaces the 3-arg `transpile_and_check!` macro.
pub fn transpile_and_check_with_exports(
    js: &str,
    test_name: &str,
    exports: HashSet<String>,
) -> String {
    let result = parse_and_transpile(js, Some(exports)).unwrap();
    println!("=== Generated Zig ({}) ===\n{}", test_name, result.zig_code);
    assert_zig_ast_check(&result.zig_code, test_name);
    result.zig_code
}

/// Assert that a "not-implemented" feature produces a `@compileError` or
/// transpiler error in the generated Zig code.
/// Either `result.errors` is non-empty, or `zig_code` contains `@compileError`.
pub fn assert_not_implemented(js: &str, feature_name: &str) {
    let result = parse_and_transpile(js, None);
    match result {
        Ok(result) => {
            // Check if there are transpiler errors
            if !result.errors.is_empty() {
                println!(
                    "[PASS] {}: transpiler returned errors: {:?}",
                    feature_name, result.errors
                );
                return;
            }
            // Check if generated code contains @compileError
            if result.zig_code.contains("@compileError") {
                println!(
                    "[PASS] {}: generated code contains @compileError",
                    feature_name
                );
                return;
            }
            // Neither: this is a problem!
            panic!(
                "[FAIL] {}: feature is marked but transpiler did NOT produce an error!\n\
                 Generated Zig code (first 500 chars):\n{}\n\
                 THIS MEANS THE TRANSPILER SILENTLY GENERATED CODE FOR AN UNSUPPORTED FEATURE.\n\
                 Please add @compileError or return an error for this feature.",
                feature_name,
                &result.zig_code[..result.zig_code.len().min(500)]
            );
        }
        Err(e) => {
            // Transpiler returned Err: also acceptable
            println!("[PASS] {}: transpiler returned Err: {}", feature_name, e);
        }
    }
}
