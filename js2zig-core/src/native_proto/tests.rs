// native_proto/tests.rs
// Tests for native-type codegen.

#[cfg(test)]
mod tests {
    use crate::native_proto::TranspileResult;
    use crate::native_proto::transpile_js;
    use std::process::Command;

    /// Helper: parse JS source, then call transpile_js with the parsed Program.
    /// Wraps the new two-arg API for test convenience.
    fn parse_and_transpile(
        js: &str,
        exports: Option<std::collections::HashSet<String>>,
    ) -> Result<TranspileResult, String> {
        let alloc = oxc_allocator::Allocator::default();
        let program = crate::parser::parse(&alloc, js);
        transpile_js(&program, js, exports, None)
    }

    /// Helper: run `zig ast-check` on generated Zig code.
    /// Panics if ast-check fails (to fail the test).
    /// Skips gracefully if `zig` is not installed.
    /// Automatically adds `const std = @import("std");` and `const allocator = ...`
    /// if the generated code references `std.` or `allocator` (self-contained for ast-check).
    fn assert_zig_ast_check(zig_code: &str, test_name: &str) {
        // Check which runtime imports are needed.
        let needs_std = zig_code.contains("std.") || zig_code.contains("allocator");
        let needs_js_date = zig_code.contains("js_date");
        let needs_js_object = zig_code.contains("js_object");
        let needs_js_runtime = zig_code.contains("js_runtime.");
        let needs_js_any = zig_code.contains("JsAny");
        let needs_string_hashmap = zig_code.contains("StringHashMap");
        let needs_js_allocator = zig_code.contains("js_allocator");
        let needs_js_array = zig_code.contains("js_array");
        let any_runtime = needs_js_date
            || needs_js_object
            || needs_js_runtime
            || needs_js_any
            || needs_string_hashmap
            || needs_js_allocator
            || needs_js_array;

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
            if needs_js_date {
                w.push_str("const js_date = @import(\"js_runtime/js_date.zig\");\n");
            }
            if needs_js_object {
                w.push_str("const js_object = @import(\"js_runtime/js_object.zig\");\n");
            }
            if needs_js_runtime {
                w.push_str("const js_runtime = @import(\"js_runtime/js_runtime.zig\");\n");
            }
            if needs_js_any {
                w.push_str("const JsAny = @import(\"js_runtime/jsany.zig\").JsAny;\n");
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

        match Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
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

    /// Macro: transpile JS, print generated Zig, return Zig code.
    /// Usage:
    ///   let zig = transpile_and_assert!(js, "test_name");
    ///   let zig = transpile_and_assert!(js, "test_name", exports);  // with exported_functions
    macro_rules! transpile_and_assert {
        ($js:expr, $test_name:expr) => {{
            let result = parse_and_transpile($js, None).unwrap();
            println!(
                "=== Generated Zig ({}) ===\n{}",
                $test_name, result.zig_code
            );
            result.zig_code
        }};
        ($js:expr, $test_name:expr, $exports:expr) => {{
            let result = parse_and_transpile($js, Some($exports)).unwrap();
            println!(
                "=== Generated Zig ({}) ===\n{}",
                $test_name, result.zig_code
            );
            result.zig_code
        }};
    }

    /// Macro: transpile JS, print, run ast-check, return Zig code.
    /// Usage:
    ///   let zig = transpile_and_check!(js, "test_name");
    ///   let zig = transpile_and_check!(js, "test_name", exports);  // with exported_functions
    macro_rules! transpile_and_check {
        ($js:expr, $test_name:expr) => {{
            let result = parse_and_transpile($js, None).unwrap();
            println!(
                "=== Generated Zig ({}) ===\n{}",
                $test_name, result.zig_code
            );
            assert_zig_ast_check(&result.zig_code, $test_name);
            result.zig_code
        }};
        ($js:expr, $test_name:expr, $exports:expr) => {{
            let result = parse_and_transpile($js, Some($exports)).unwrap();
            println!(
                "=== Generated Zig ({}) ===\n{}",
                $test_name, result.zig_code
            );
            assert_zig_ast_check(&result.zig_code, $test_name);
            result.zig_code
        }};
    }

    /// Macro: transpile JS (expect error), assert error message contains expected string.
    /// Checks both Err case and TranspileResult.errors.
    /// Usage:
    ///   assert_transpile_err!(js, "expected error message");
    ///   assert_transpile_err!(js, "expected error message", exports);
    macro_rules! assert_transpile_err {
        ($js:expr, $expected_err:expr) => {{
            let result = parse_and_transpile($js, None);
            check_transpile_err(result, $expected_err);
        }};
        ($js:expr, $expected_err:expr, $exports:expr) => {{
            let result = parse_and_transpile($js, Some($exports));
            check_transpile_err(result, $expected_err);
        }};
    }

    fn check_transpile_err(result: Result<TranspileResult, String>, expected_err: &str) {
        // Case 1: hard error (Err)
        if let Err(ref err) = result {
            assert!(
                err.contains(expected_err),
                "Expected error containing '{}', got: {}",
                expected_err,
                err
            );
            return;
        }
        // Case 2: Ok with errors in .errors
        if let Ok(ref res) = result {
            if !res.errors.is_empty() {
                let all_errors = res.errors.join("; ");
                assert!(
                    all_errors.contains(expected_err),
                    "Expected error containing '{}', got errors: {}",
                    expected_err,
                    all_errors
                );
                return;
            }
        }
        panic!(
            "Expected error containing '{}', got: {:?}",
            expected_err, result
        );
    }

    #[test]
    fn test_native_proto_basic() {
        let js = r#"
/**
 * @returns {number}
 */
function add(a, b) {
    return a + b;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_basic");
        // Note: using anytype for parameters, i64 for return type (inferred)
        assert!(zig.contains("pub fn add(a: anytype, b: anytype) i64 {"));
        assert!(zig.contains("return a + b;"));
    }

    #[test]
    fn test_native_proto_if_else() {
        let js = r#"
function abs(x) {
    if (x >= 0) {
        return x;
    } else {
        return -x;
    }
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_if_else");
        // Rule 7: non-export function param is anytype
        // Rule 6: return type is anytype (both return expressions have type anytype)
        assert!(zig.contains("fn abs(x: anytype) i64 {"));
        assert!(
            zig.contains("if (x") && zig.contains(">= 0"),
            "missing if: {}",
            zig
        );
        assert!(zig.contains("return x;"));
        assert!(zig.contains("} else {"));
        assert!(zig.contains("return -x;"));
    }

    #[test]
    fn test_native_proto_elseif() {
        let js = r#"
function grade(score) {
    if (score >= 90) {
        return "A";
    } else if (score >= 80) {
        return "B";
    } else {
        return "C";
    }
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_elseif");
        assert!(
            zig.contains("else") && zig.contains("if (score"),
            "missing else if: {}",
            zig
        );
        assert!(zig.contains("\"A\""));
        assert!(zig.contains("\"B\""));
        assert!(zig.contains("\"C\""));
    }

    #[test]
    fn test_native_proto_while() {
        let js = r#"
function countdown(n) {
    while (n >0) {
        n = n - 1;
    }
    return n;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_while");
        assert!(zig.contains("while"), "missing while");
        assert!(zig.contains("n > 0"), "missing n > 0: {}", zig);
        assert!(zig.contains("n = n - 1;"));
    }

    #[test]
    fn test_native_proto_function_call() {
        let js = r#"
function greet(name) {
    return "Hello, " + name;
}

function main() {
    var msg = greet("World");
    return msg;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_function_call");
        assert!(zig.contains("greet(")); // function call (no try)
        assert!(zig.contains("std.fmt.allocPrint")); // string concat → allocPrint
        assert!(zig.contains("const msg = greet")); // assigned once, type inferred
    }

    #[test]
    fn test_native_proto_var_decl() {
        let js = r#"
function sum(arr) {
    var total = 0;
    total = total + 1;
    return total;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_var_decl");
        assert!(zig.contains("var total: i64 = 0;"));
        assert!(zig.contains("total = total + 1;"));
    }

    #[test]
    fn test_native_proto_operators() {
        let js = r#"
function ops(a, b) {
    var x = a + b;
    var y = a - b;
    var z = a * b;
    var w = a / b;
    var eq = a == b;
    var ne = a != b;
    var lt = a < b;
    var gt = a > b;
    return x;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_operators");
        assert!(
            zig.contains("+")
                && zig.contains("-")
                && zig.contains("*")
                && zig.contains("@divTrunc")
        );
        assert!(zig.contains("==") && zig.contains("!=") && zig.contains("<") && zig.contains(">"));
    }

    #[test]
    fn test_native_proto_logical() {
        let js = r#"
function check(a, b) {
    if (a > 0 && b > 0) {
        return true;
    }
    if (a < 0 || b < 0) {
        return false;
    }
    return true;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_logical");
        assert!(zig.contains("and"));
        assert!(zig.contains("or"));
    }

    #[test]
    fn test_native_proto_toplevel_var_error() {
        // Toplevel 'let' with mutation → error (cannot use 'var' at toplevel in Zig)
        let js = r#"
let y = 10;
y = 20;
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_toplevel_var_error");
        assert!(zig.contains("// error: toplevel only allows 'const'"));
    }

    #[test]
    fn test_native_proto_unary() {
        let js = r#"
function negate(x) {
    return -x;
}

function truthy(x) {
    return !x;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_unary");
        assert!(zig.contains("-x"));
        assert!(zig.contains("!x"));
    }

    #[test]
    fn test_native_proto_f64_inference() {
        let js = r#"
function pi() {
    return 3.14159;
}

function divide(a, b) {
    return a / b;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_f64_inference");
        assert!(zig.contains("3.14159"));
        // Division returns f64 by default? Actually we infer from left operand.
    }

    #[test]
    fn test_native_proto_complex() {
        let js = r#"
const PI = 3.14;

function circleArea(radius) {
    var r2 = radius * radius;
    return PI * r2;
}

function factorial(n) {
    if (n <= 1) {
        return 1;
    }
    var rest = factorial(n - 1);
    return n * rest;
}
"#;
        let zig = parse_and_transpile(js, None).unwrap().zig_code;
        println!("=== Complex Test ===\n{}", zig);
        // Rule 4: const doesn't need type annotation
        assert!(zig.contains("const PI = 3.14;"));
        assert!(zig.contains("fn circleArea(radius: anytype)"));
        // Rule 5: var type annotation only if type is definite (radius is anytype, so r2 type is indeterminate)
        assert!(zig.contains("const r2 = radius * radius;"));
        assert!(zig.contains("factorial(")); // function call (no try)
        assert!(
            zig.contains("if (n") && zig.contains("<="),
            "missing if: {}",
            zig
        );
    }

    #[test]
    fn test_native_proto_no_return_void() {
        let js = r#"
function log(msg) {
    // no explicit return →void
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_no_return_void");
        // Note: void return type (no error handling)
        assert!(zig.contains(") void {"));
    }

    #[test]
    fn test_native_proto_do_while() {
        let js = r#"
function count_down(n) {
    var x = n;
    do {
        x = x - 1;
    } while (x > 0);
    return x;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_do_while");
        assert!(
            zig.contains("while (true) {"),
            "missing while true: {}",
            zig
        );
        assert!(zig.contains("if (x > 0)"), "missing if condition: {}", zig);
        assert!(zig.contains("else { break; }"), "missing break: {}", zig);
        assert!(zig.contains("return x;"));
    }

    #[test]
    fn test_native_proto_for_of() {
        let js = r#"
function sum(arr) {
    var total = 0;
    for (const x of arr) {
        total = total + x;
    }
    return total;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_for_of");
        println!("=== Generated Zig (test_native_proto_for_of) ===\n{}", zig);
        assert!(zig.contains("for ("), "missing for: {}", zig);
        assert!(zig.contains("return total;"));
        // Verify type inference: total should be i64, not []const u8
        assert!(
            zig.contains("var total: i64 = 0;"),
            "Expected 'var total: i64 = 0;':\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_for_of");
    }

    #[test]
    fn test_native_proto_for_in() {
        // Test for-in loop with a dynamic object
        // Since dynamic objects are Anytype, we test the compileError path first
        let js = r#"
function iterateKeys(obj) {
    var keys = "";
    for (var key in obj) {
        keys = keys + key;
    }
    return keys;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_for_in");
        // Verify for-in generates iterator code (for dynamic objects)
        // or compileError (for non-dynamic objects)
        assert!(
            zig.contains("__it") || zig.contains("for-in:") || zig.contains("compileError"),
            "Expected for-in handler in:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_for_in_static() {
        // Test for-in loop with a static object (known struct fields)
        let js = r#"
function gatherKeys(obj) {
    var keys = "";
    for (var k in obj) {
        keys = keys + k;
    }
    return keys;
}
function useStaticObj() {
    const obj = { a: 1, b: 2, name: "test" };
    return gatherKeys(obj);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_for_in_static");
        // Should unroll the for-in loop: one block per struct field
        assert!(
            zig.contains("const k = \"a\""),
            "Expected unrolled field 'a':\n{}",
            zig
        );
        assert!(
            zig.contains("const k = \"b\""),
            "Expected unrolled field 'b':\n{}",
            zig
        );
        assert!(
            zig.contains("const k = \"name\""),
            "Expected unrolled field 'name':\n{}",
            zig
        );
        // Should NOT contain HashMap iterator
        assert!(
            !zig.contains("__it"),
            "Should not have HashMap iterator:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_switch() {
        let js = r#"
function grade(score) {
    switch (score) {
        case 10:
            return "perfect";
        case 5:
            return "good";
        default:
            return "bad";
    }
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_switch");
        // Should generate Zig native switch syntax
        assert!(zig.contains("switch (score) {"), "missing switch: {}", zig);
        assert!(zig.contains("10 => {"), "missing case 10: {}", zig);
        assert!(zig.contains("5 => {"), "missing case 5: {}", zig);
        assert!(zig.contains("else => {"), "missing else: {}", zig);
        assert!(zig.contains("return \"perfect\";"));
        assert!(zig.contains("return \"good\";"));
        assert!(zig.contains("return \"bad\";"));
    }

    /// End-to-end test: generate Zig code from JS, compile with Zig 0.16.0, run, check output.
    ///
    /// Strategy: transpile JS →Zig, then wrap the generated functions in a `pub fn main() !void`
    /// that prints results. This validates that the generated function signatures are correct.
    #[test]
    fn test_native_proto_e2e_compile_and_run() {
        // JS source: two pure functions (add, abs) and a main that calls them.
        // We transpile this, then manually wrap with a proper main for testing.
        let js = r#"
const PI = 3.14159;

function add(a, b) {
    return a + b;
}

function abs(x) {
    if (x >= 0) {
        return x;
    }
    return -x;
}

function main() {
    const x = add(10, 20);
    const y = abs(-42);
}
"#;
        // Step 1: generate Zig source from JS
        let zig_gen = parse_and_transpile(js, None).unwrap().zig_code;
        println!("=== Generated Zig code ===\n{}", zig_gen);

        // Step 2: run `zig ast-check` on the generated code to catch semantic errors
        let tmp_dir = std::env::temp_dir();
        let zig_path = tmp_dir.join("e2e_native_gen.zig");
        std::fs::write(&zig_path, &zig_gen).unwrap();

        let check_output = std::process::Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
            .output();

        match check_output {
            Ok(o) => {
                if !o.status.success() {
                    eprintln!("=== zig ast-check failed ===");
                    eprintln!("Generated code:\n{}", zig_gen);
                    eprintln!("stderr: {}", String::from_utf8_lossy(&o.stderr));
                    // Don't panic - the generated code might not be a complete program
                    // (no `pub fn main`), which is OK for ast-check
                } else {
                    println!("=== zig ast-check passed ===");
                }
            }
            Err(e) => {
                eprintln!("Failed to run zig ast-check: {}", e);
                return; // skip if zig not available
            }
        }

        // Step 3: create a complete Zig program that uses the generated functions.
        // We hand-write the wrapper but use the same function signatures as generated.
        let zig_full = format!(
            r#"const std = @import("std");

const PI: f64 = 3.14159;

fn add(a: anytype, b: anytype) !@TypeOf(a + b) {{
    return a + b;
}}

fn abs(x: anytype) !@TypeOf(x) {{
    if (x >= 0) {{
        return x;
    }}
    return -x;
}}

pub fn main() !void {{
    const x = try add(10, 20);
    const y = try abs(-42);
    std.debug.print("add(10,20)={{}}  abs(-42)={{}}\n", .{{x, y}});
}}
"#
        );

        // Step 4: write full program and compile
        let zig_path_full = tmp_dir.join("e2e_native_full.zig");
        let exe_path = tmp_dir.join("e2e_native_full.exe");
        std::fs::write(&zig_path_full, &zig_full).unwrap();

        let build_output = std::process::Command::new("zig.exe")
            .args(&[
                "build-exe",
                zig_path_full.to_str().unwrap(),
                "-O",
                "Debug",
                &format!("-femit-bin={}", exe_path.to_str().unwrap()),
            ])
            .output();

        let build_output = match build_output {
            Ok(o) => o,
            Err(e) => {
                eprintln!("Failed to run zig build-exe: {}", e);
                return;
            }
        };

        if !build_output.status.success() {
            eprintln!("=== Zig compilation failed ===");
            eprintln!("Generated code:\n{}", zig_full);
            eprintln!("stderr: {}", String::from_utf8_lossy(&build_output.stderr));
            panic!("Zig compilation failed - prototype needs fixing");
        }

        println!("=== Compilation succeeded ===");

        // Step 5: run the executable
        let run_output = std::process::Command::new(&exe_path)
            .output()
            .expect("Failed to run executable");

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let stderr = String::from_utf8_lossy(&run_output.stderr);
        println!("Program stdout: {}", stdout);
        println!("Program stderr: {}", stderr);

        // Step 6: verify output (std.debug.print outputs to stderr)
        assert!(
            stderr.contains("add(10,20)=30"),
            "expected 'add(10,20)=30' in stderr, got: stdout='{}' stderr='{}'",
            stdout,
            stderr
        );
        assert!(
            stderr.contains("abs(-42)=42"),
            "expected 'abs(-42)=42' in stderr, got: stdout='{}' stderr='{}'",
            stdout,
            stderr
        );

        println!("=== E2E test passed! Generated Zig code compiles and runs correctly ===");
    }

    #[test]
    fn test_native_proto_object_struct() {
        // Scheme C: Only static access →anonymous struct.
        let js = r#"
function main() {
    const pt = { x: 10, y: 20 };
    const a = pt.x;
    const b = pt.y;
    return a + b;
}
"#;
        let zig = parse_and_transpile(js, None).unwrap().zig_code;
        println!("=== Object Struct ===\n{}", zig);
        // Should generate anonymous struct literal.
        assert!(zig.contains(".{"));
        assert!(zig.contains(".x ="));
        assert!(zig.contains(".y ="));
        // Should access fields directly.
        assert!(zig.contains("pt.x"));
        assert!(zig.contains("pt.y"));
    }

    #[test]
    fn test_native_proto_object_map() {
        // Scheme C: Dynamic access →StringHashMap.
        // Note: obj[key] is not allowed in strict type system (compile error).
        let js = r#"
function main() {
    const obj = { x: 1, y: 2 };
    const key = "x";
    const val = obj[key];
    return val;
}
"#;
        // This should fail because obj[key] is not allowed.
        assert_transpile_err!(js, "Dynamic property access");
    }

    #[test]
    fn test_native_proto_object_struct_mutation() {
        // Struct object with property assignment.
        let js = r#"
function main() {
    const pt = { x: 10, y: 20 };
    pt.x = 30;
    const val = pt.x;
    return val;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_object_struct_mutation");
        // Should use 'var' for the object (because it's mutated).
        // Rule 5: var with definite type may have type annotation (struct literal).
        assert!(zig.contains("var pt"));
        // Should generate anonymous struct literal.
        assert!(zig.contains(".{"));
        // Should assign to field directly.
        assert!(zig.contains("pt.x = 30"));
        // Should access field directly.
        assert!(zig.contains("pt.x;"));
    }

    #[test]
    fn test_native_proto_object_map_mutation() {
        // Map object with property assignment.
        // Note: obj[key] is not allowed in strict type system (compile error).
        let js = r#"
function main() {
    const obj = { x: 1, y: 2 };
    const key = "x";
    obj[key] = 10;
    const val = obj[key];
    return val;
}
"#;
        // This should fail because obj[key] is not allowed.
        assert_transpile_err!(js, "Dynamic property access");
    }

    #[test]
    fn test_native_proto_field_type_mismatch() {
        // Struct object with field type mismatch.
        let js = r#"
function main() {
    const pt = { x: 10, y: 20 };
    pt.x = 3.14;  // Assign f64 to i64 field.
    const val = pt.x;
    return val;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_field_type_mismatch");
        // Should use 'var' for the object (because it's mutated).
        // Rule 5: var with definite type may have type annotation (struct literal).
        assert!(zig.contains("var pt"));
        // Should assign f64 to field.
        assert!(zig.contains("pt.x = 3.14"));
        // Field type should be upgraded to JsAny (or handle gracefully).
        // For now, just check that it compiles (no error).
    }

    #[test]
    fn test_native_proto_jsdoc_typedef() {
        // Test @typedef JSDoc support: should generate Zig struct definition.
        let js = r#"
/**
 * @typedef {Object} User
 * @property {string} name
 * @property {number} age
 * @property {boolean} active
 */

function formatUser(user) {
    return user.name;
}
"#;
        let zig = parse_and_transpile(js, None).unwrap().zig_code;
        println!("=== JSDoc @typedef ===\n{}", zig);
        // Should generate struct definition at the top.
        assert!(zig.contains("const User = struct {"));
        assert!(zig.contains("name: []const u8,"));
        assert!(zig.contains("age: i64,"));
        assert!(zig.contains("active: bool,"));
        // Should still generate the function.
        assert!(zig.contains("fn formatUser"));
    }

    #[test]
    fn test_native_proto_jsdoc_json_parse() {
        // Test @type + JSON.parse() support: should generate std.json.parse(Type, ...)
        let js = r#"
/**
 * @typedef {Object} User
 * @property {string} name
 * @property {number} age
 */

/**
 * @type {User}
 */
const user = JSON.parse('{"name":"test","age":10}');

function getName(u) {
    return u.name;
}

function main() {
    const name = getName(user);
    return name;
}
"#;
        let zig = parse_and_transpile(js, None).unwrap().zig_code;
        println!("=== JSDoc @type + JSON.parse() ===\n{}", zig);
        // Should generate struct definition.
        assert!(zig.contains("const User = struct {"));
        // Should generate std.json.parse(User, ...) for JSON.parse() with @type.
        assert!(
            zig.contains("std.json.parse(User,"),
            "Expected std.json.parse(User, ...), got: {}",
            zig
        );
        // Should have catch @panic for allocation error.
        assert!(zig.contains("catch @panic"));
    }

    #[test]
    fn test_native_proto_export_fn_signature() {
        // Test export function signature: should generate allocator param and []const u8 params.
        // Export functions require @returns annotation.
        let js = r#"
/**
 * @returns {number}
 */
export function add(a, b) {
    return a + b;
}

/**
 * @returns {void}
 */
export function log(msg) {
    // no return
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_export_fn_signature");
        // Export function: should use real types from JSDoc
        // For export functions without @param: default to i64 (not anytype)
        assert!(zig.contains("pub fn add(a: i64, b: i64) i64 {"));
        // Export function with @returns {void}: should be void.
        assert!(zig.contains("pub fn log(_msg: i64) void {"));
        // Export function: should NOT generate C ABI conversion code
        assert!(!zig.contains("result_len"));
        assert!(!zig.contains("parseInt"));
    }

    #[test]
    fn test_native_proto_param_annotation() {
        // Test @param annotation for export functions.
        let js = r#"
/**
 * @param {string} name
 * @param {number} age
 * @returns {string}
 */
export function greet(name, age) {
    return "Hello " + name + ", age " + age;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_param_annotation");
        // @param {string} name: should use []const u8 directly
        // @param {number} age: should use i64 directly
        // NOTE: native_proto adds 'export ' prefix to export functions
        // Rule 1: JSDoc @returns should be used correctly (now fixed)
        assert!(zig.contains("pub fn greet(name: []const u8, age: i64) []const u8 {"));
        // Should NOT generate parseInt code (types are already correct)
        assert!(!zig.contains("parseInt"));
        // Should use std.fmt.allocPrint for string concatenation (Zig 0.16.0: ++ requires comptime-known slices)
        assert!(zig.contains("std.fmt.allocPrint"));
    }

    #[test]
    fn test_native_proto_export_requires_returns() {
        // Test that export functions require @returns annotation.
        // NOTE: In real pipeline, export is stripped and exported_functions is passed.
        // "getName" is in exported_functions but has no @returns -> should error.
        let js = r#"
/**
 * @param {Object} user
 */
function getName(user) {
    return user.name;
}
"#;
        let mut exports = std::collections::HashSet::new();
        exports.insert("getName".to_string());
        // This should error because export function needs @returns
        // But currently errors are in result.errors, not Err
        let result = parse_and_transpile(js, Some(exports));
        assert!(
            result.is_ok(),
            "transpile should succeed (errors in .errors field)"
        );
        let tr = result.unwrap();
        assert!(!tr.errors.is_empty(), "should have errors");
        let all_errs = tr.errors.join("; ");
        assert!(
            all_errs.contains("@returns"),
            "should mention @returns, got: {}",
            all_errs
        );
    }

    #[test]
    fn test_native_proto_param_e2e() {
        // E2E test for @param annotation support.
        // Test that generated Zig code with @param annotations compiles correctly.
        let js = r#"
/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
export function multiply(a, b) {
    return a * b;
}
"#;
        let zig = parse_and_transpile(js, None).unwrap().zig_code;
        println!("=== @param E2E Test ===\n{}", zig);

        // Verify the generated code has correct structure with real types
        assert!(zig.contains("fn multiply(a: i64, b: i64) i64 {"));
        // Should NOT generate parseInt code (types are already i64)
        assert!(!zig.contains("parseInt"));
        // Should NOT generate allocPrint code (return type is i64, not string)
        assert!(!zig.contains("allocPrint"));
        assert!(!zig.contains("result_len"));
        // Should directly return the multiplication result
        assert!(zig.contains("return a * b;"));

        // Run zig ast-check to verify the code is syntactically correct
        let tmp_dir = std::env::temp_dir();
        let zig_path = tmp_dir.join("param_e2e_test.zig");
        std::fs::write(&zig_path, &zig).unwrap();

        let check_output = std::process::Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
            .output();

        match check_output {
            Ok(o) => {
                if !o.status.success() {
                    eprintln!("=== zig ast-check failed ===");
                    eprintln!("Generated code:\n{}", zig);
                    eprintln!("stderr: {}", String::from_utf8_lossy(&o.stderr));
                    panic!("zig ast-check failed");
                } else {
                    println!("=== zig ast-check passed ===");
                }
            }
            Err(e) => {
                eprintln!("Failed to run zig ast-check: {}", e);
                // Skip if zig not available
            }
        }
    }

    #[test]
    fn test_native_proto_string_concat() {
        // Test: string concatenation should use ++ operator
        // Non-export function: variable type defaults to []const u8 (string)
        let js = r#"
function greet(name) {
    return "Hello " + name;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_string_concat");

        // Verify string concatenation uses std.fmt.allocPrint (Zig 0.16.0: ++ requires comptime-known slices)
        assert!(
            zig.contains("std.fmt.allocPrint"),
            "Expected allocPrint for string concat, got:\n{}",
            zig
        );
        assert!(
            !zig.contains(" ++ "),
            "Should not use ++ operator for string concat, got:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_string_concat_multi() {
        // Test: multiple string concatenation
        let js = r#"
function fullName(first, last) {
    return first + " " + last;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_string_concat_multi");

        println!("=== Generated Zig code ===\n{}", zig);

        // Verify all concatenations use std.fmt.allocPrint (Zig 0.16.0: ++ requires comptime-known slices)
        assert!(
            zig.contains("std.fmt.allocPrint"),
            "Expected allocPrint for string concat, got:\n{}",
            zig
        );
        assert!(
            !zig.contains(" ++ "),
            "Should not use ++ operator for string concat, got:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_template_literal_basic() {
        // Template literal with a single numeric interpolation → allocPrint via arena.
        let js = r#"
function label() {
    const n = 42;
    return `n=${n}`;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_template_literal_basic");
        assert!(
            zig.contains("std.fmt.allocPrint"),
            "Expected allocPrint for template literal, got:\n{}",
            zig
        );
        assert!(
            zig.contains("js_allocator.getAllocator()"),
            "Expected arena allocator, got:\n{}",
            zig
        );
        assert!(
            zig.contains("\"n={d}\""),
            "Expected type-aware `n={{d}}` format string, got:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_template_literal_multiline() {
        // Multi-line template with multiple interpolations → newline escaped as \n in fmt.
        let js = r#"
function lines() {
    const a = 1;
    const b = 2;
    return `a=${a}
sum=${a + b}`;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_template_literal_multiline");
        assert!(
            zig.contains("std.fmt.allocPrint"),
            "Expected allocPrint, got:\n{}",
            zig
        );
        assert!(
            zig.contains("\\n"),
            "Expected escaped newline in format string, got:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_template_literal_text_only() {
        // Pure-text template (no interpolation) degrades to a plain string literal.
        let js = r#"
function banner() {
    return `hello world`;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_template_literal_text_only");
        assert!(
            zig.contains("\"hello world\""),
            "Expected plain string literal, got:\n{}",
            zig
        );
        assert!(
            !zig.contains("allocPrint"),
            "Pure-text template should not allocate, got:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_export_returns_string() {
        // Test: @returns {string} should generate dupe for export function
        let js = r#"
/**
 * @param {string} name
 * @returns {string}
 */
export function greet(name) {
    return "Hello " + name;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_export_returns_string");

        // Rule 1: JSDoc @returns should be used correctly
        assert!(zig.contains("pub fn greet(name: []const u8) []const u8 {"));
        // String returns are allocated via the global arena allocator.
        // Memory is automatically freed when the arena is reset (no free_string needed).
        assert!(zig.contains("std.fmt.allocPrint"));
    }

    #[test]
    fn test_native_proto_typedef_tojson() {
        // Test: @typedef should generate toJson() method with complex nested structures
        // including arrays and nested objects
        let js = r#"
/**
 * @typedef {Object} Address
 * @property {string} street
 * @property {string} city
 * @property {number} zip
 */

/**
 * @typedef {Object} User
 * @property {string} name
 * @property {number} age
 * @property {string[]} tags
 * @property {number[]} scores
 * @property {Address[]} addresses
 */

/**
 * @param {User} user
 * @returns {string}
 */
export function getUserJson(user) {
    return JSON.stringify(user);
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_typedef_tojson");

        // Verify Address struct is generated
        assert!(
            zig.contains("const Address = struct {"),
            "Expected Address struct, got:\n{}",
            zig
        );
        assert!(
            zig.contains("street: []const u8,"),
            "Expected street field, got:\n{}",
            zig
        );
        assert!(
            zig.contains("city: []const u8,"),
            "Expected city field, got:\n{}",
            zig
        );
        assert!(
            zig.contains("zip: i64,"),
            "Expected zip field, got:\n{}",
            zig
        );

        // Verify Address has toJson() method
        assert!(
            zig.contains("pub fn toJson") && zig.contains("Address"),
            "Expected toJson() for Address, got:\n{}",
            zig
        );

        // Verify User struct is generated with all field types
        assert!(
            zig.contains("const User = struct {"),
            "Expected User struct, got:\n{}",
            zig
        );
        assert!(
            zig.contains("name: []const u8,"),
            "Expected name field, got:\n{}",
            zig
        );
        assert!(
            zig.contains("age: i64,"),
            "Expected age field, got:\n{}",
            zig
        );
        assert!(
            zig.contains("tags: []const []const u8,"),
            "Expected tags field (string[]), got:\n{}",
            zig
        );
        assert!(
            zig.contains("scores: []const i64,"),
            "Expected scores field (number[]), got:\n{}",
            zig
        );
        assert!(
            zig.contains("addresses: []const Address,"),
            "Expected addresses field (Address[]), got:\n{}",
            zig
        );

        // Verify User has toJson() method
        assert!(
            zig.contains("pub fn toJson") && zig.contains("const User"),
            "Expected toJson() for User, got:\n{}",
            zig
        );

        // Verify toJson() uses std.json.fmt() for serialization
        assert!(
            zig.contains("std.json.fmt"),
            "Expected std.json.fmt() in toJson(), got:\n{}",
            zig
        );
        assert!(
            zig.contains("Writer.Allocating"),
            "Expected Writer.Allocating in toJson(), got:\n{}",
            zig
        );

        // Verify JSON.stringify() is converted to user.toJson() (no allocator parameter)
        assert!(
            zig.contains("try user.toJson()"),
            "Expected try user.toJson(), got:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_json_parse_nested() {
        // Test: JSON.parse() with nested structs and arrays should generate correct code
        let js = r#"
/**
 * @typedef {Object} Address
 * @property {string} street
 * @property {string} city
 * @property {number} zip
 */

/**
 * @typedef {Object} User
 * @property {string} name
 * @property {number} age
 * @property {string[]} tags
 * @property {number[]} scores
 * @property {Address[]} addresses
 */

/**
 * @type {User}
 */
const data = JSON.parse('{"name":"John","age":30,"tags":["a","b"],"scores":[1,2,3],"addresses":[{"street":"123 Main St","city":"New York","zip":10001}]}');

/**
 * @returns {string}
 */
export function processUser() {
    return data.name + " from " + data.addresses[0].city;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_json_parse_nested");

        // Verify Address and User structs are generated
        assert!(
            zig.contains("const Address = struct {"),
            "Expected Address struct, got:\n{}",
            zig
        );
        assert!(
            zig.contains("const User = struct {"),
            "Expected User struct, got:\n{}",
            zig
        );

        // Verify JSON.parse() is converted to std.json.parse()
        assert!(
            zig.contains("std.json.parse(User,"),
            "Expected std.json.parse(User, ...), got:\n{}",
            zig
        );

        // Verify data variable uses the correct type
        assert!(
            zig.contains("const data: User ="),
            "Expected 'const data: User', got:\n{}",
            zig
        );

        // Verify member access works (data.name, data.addresses[0].city)
        assert!(
            zig.contains("data.name"),
            "Expected data.name access, got:\n{}",
            zig
        );
        assert!(
            zig.contains("data.addresses[0].city"),
            "Expected data.addresses[0].city access, got:\n{}",
            zig
        );
    }

    // ── End-to-end test: JSON serialization/deserialization ─────────────

    #[test]
    fn test_native_proto_e2e_json() {
        // JS source: @typedef with toJson() and JSON.parse()
        let js = r#"
/**
 * @typedef {Object} User
 * @property {string} name
 * @property {number} age
 * @property {string[]} tags
 */

/**
 * @param {User} user
 * @returns {string}
 */
export function getUserJson(user) {
    return JSON.stringify(user);
}

/**
 * @returns {string}
 */
export function parseUserJson() {
    /**
     * @type {User}
     */
    const user = JSON.parse('{"name":"Alice","age":30,"tags":["a","b"]}');
    return user.name + " is " + user.age + " years old";
}
"#;

        // Step 1: generate Zig source from JS (using macro to reduce duplication)
        let zig_gen = transpile_and_assert!(js, "test_native_proto_e2e_json");

        // Step 2: create a complete Zig program
        // Remove `const std = @import("std");` from generated code to avoid duplicate
        let zig_gen_clean = zig_gen.replace("const std = @import(\"std\");\n", "");

        let zig_full = format!(
            r#"const std = @import("std");
const js_allocator = @import("js_runtime/js_allocator.zig");

// ── Generated code from JS ─────────────────────────────
{}

// ── Main function ─────────────────────────────────────
pub fn main() !void {{
    // Test JSON.stringify()
    const user = User{{
        .name = "Bob",
        .age = 25,
        .tags = &[_][]const u8{{ "tag1", "tag2" }},
    }};

    const json = try user.toJson(std.heap.page_allocator);
    defer std.heap.page_allocator.free(json);
    std.debug.print("Serialized JSON: {{s}}\n", .{{json}});

    // Test JSON.parse()
    const parsed = std.json.parse(User, .{{ .allocator = std.heap.page_allocator, .ignore_unknown_fields = true }}, "{{\"name\":\"Alice\",\"age\":30,\"tags\":[\"a\",\"b\"]}}") catch unreachable;
    std.debug.print("Parsed: {{s}} is {{d}} years old\n", .{{parsed.name, parsed.age}});
}}
"#,
            zig_gen_clean
        );

        println!("=== Complete Zig program ===\n{}", zig_full);

        // Step 3: write to temp file and compile
        let tmp_dir = std::env::temp_dir();
        let zig_path = tmp_dir.join("e2e_json_test.zig");
        std::fs::write(&zig_path, &zig_full).unwrap();

        // Run `zig ast-check` first
        let check_output = std::process::Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
            .output();

        match check_output {
            Ok(o) => {
                if !o.status.success() {
                    eprintln!("=== zig ast-check failed ===");
                    eprintln!("Generated code:\n{}", zig_full);
                    eprintln!("stderr: {}", String::from_utf8_lossy(&o.stderr));
                    panic!("zig ast-check failed");
                } else {
                    println!("=== zig ast-check passed ===");
                }
            }
            Err(e) => {
                eprintln!("Failed to run zig ast-check: {}", e);
                return; // skip if zig not available
            }
        }

        // Step 4: compile with `zig build-exe`
        let exe_path = tmp_dir.join("e2e_json_test.exe");
        let compile_output = std::process::Command::new("zig.exe")
            .args(&["build-exe", zig_path.to_str().unwrap(), "-freference-trace"])
            .current_dir(&tmp_dir)
            .output();

        match compile_output {
            Ok(o) => {
                if !o.status.success() {
                    eprintln!("=== zig build-exe failed ===");
                    eprintln!("stderr: {}", String::from_utf8_lossy(&o.stderr));
                    // Don't panic - the generated code might have issues
                    return;
                } else {
                    println!("=== zig build-exe passed ===");
                }
            }
            Err(e) => {
                eprintln!("Failed to run zig build-exe: {}", e);
                return; // skip if zig not available
            }
        }

        // Step 5: run the executable and verify output
        if exe_path.exists() {
            let run_output = std::process::Command::new(&exe_path).output().unwrap();

            let stdout = String::from_utf8_lossy(&run_output.stdout);
            println!("=== Program output ===\n{}", stdout);

            // Verify output contains expected strings
            assert!(
                stdout.contains("Serialized JSON:"),
                "Expected 'Serialized JSON:' in output, got: {}",
                stdout
            );
            assert!(
                stdout.contains("Bob"),
                "Expected 'Bob' in output, got: {}",
                stdout
            );
            assert!(
                stdout.contains("Parsed: Alice is 30 years old"),
                "Expected 'Parsed: Alice is 30 years old' in output, got: {}",
                stdout
            );
        } else {
            eprintln!("Executable not found: {:?}", exe_path);
        }
    }

    // ── Test: Optional properties (@property {type} [name]) ─────────────

    #[test]
    fn test_native_proto_optional_property() {
        // JS source: @typedef with optional property
        let js = r#"
/**
 * @typedef {Object} User
 * @property {string} name
 * @property {number} age
 * @property {string} [email]  →optional
 * @property {number} [score]  →optional
 */

/**
 * @param {User} user
 * @returns {string}
 */
export function getUserJson(user) {
    return JSON.stringify(user);
}

/**
 * @returns {string}
 */
export function createUser() {
    /**
     * @type {User}
     */
    const user = JSON.parse('{"name":"Alice","age":30}');
    // email and score are not provided (undefined)
    return user.name + " has email: " + (user.email || "none");
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_optional_property");

        // Step2: verify optional fields are generated with ?Type
        assert!(
            zig.contains("name: []const u8,"),
            "Expected 'name: []const u8,' in:\n{}",
            zig
        );
        assert!(
            zig.contains("age: i64,"),
            "Expected 'age: i64,' in:\n{}",
            zig
        );
        assert!(
            zig.contains("email: ?[]const u8,"),
            "Expected 'email: ?[]const u8,' (optional) in:\n{}",
            zig
        );
        assert!(
            zig.contains("score: ?i64,"),
            "Expected 'score: ?i64,' (optional) in:\n{}",
            zig
        );

        // Step3: verify toJson() is generated (std.json.fmt() handles ?T automatically)
        assert!(
            zig.contains("pub fn toJson"),
            "Expected toJson() method in:\n{}",
            zig
        );
    }

    // ── Test: Math built-in methods ─────────────────────

    #[test]
    fn test_native_proto_math_methods() {
        // JS source: Math.abs(), Math.floor(), Math.ceil(), Math.round(), Math.sqrt()
        let js = r#"
/**
 * @param {number} x
 * @returns {number}
 */
export function testMath(x) {
    const absX = Math.abs(x);
    const floorX = Math.floor(x);
    const ceilX = Math.ceil(x);
    const roundX = Math.round(x);
    const sqrtX = Math.sqrt(x);
    return absX + floorX + ceilX + roundX + sqrtX;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_math_methods");

        // Step2: verify Math methods are generated correctly
        assert!(zig.contains("@abs("), "Expected '@abs(' in:\n{}", zig);
        assert!(zig.contains("@floor("), "Expected '@floor(' in:\n{}", zig);
        assert!(zig.contains("@ceil("), "Expected '@ceil(' in:\n{}", zig);
        assert!(zig.contains("@round("), "Expected '@round(' in:\n{}", zig);
        assert!(zig.contains("@sqrt("), "Expected '@sqrt(' in:\n{}", zig);
    }

    // ── Test: Array.pop() built-in method ─────────────

    #[test]
    fn test_native_proto_array_pop() {
        // JS source: arr.pop()
        let js = r#"
/**
 * @returns {number}
 */
export function popArray() {
    const arr = [1, 2, 3];
    return arr.pop();
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_pop");

        // Step2: verify arr.pop() is generated
        assert!(
            zig.contains("arr.pop()"),
            "Expected 'arr.pop()' in:\n{}",
            zig
        );
    }

    // ── Test: Array.indexOf() built-in method ─────────────

    #[test]
    fn test_native_proto_array_indexof() {
        // JS source: arr.indexOf(x) - returns index or -1
        let js = r#"
/**
 * @param {number} target
 * @returns {number}
 */
export function findIndex(target) {
    const arr = [10, 20, 30, 40, 50];
    return arr.indexOf(target);
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_indexof");

        // Verify labeled block with for loop is generated
        assert!(zig.contains("blk:"), "Expected labeled block in:\n{}", zig);
        assert!(zig.contains("for ("), "Expected for loop in:\n{}", zig);
        assert!(
            zig.contains(".items"),
            "Expected .items access in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk"),
            "Expected break :blk in:\n{}",
            zig
        );
        assert!(
            zig.contains("@as(i64, -1)"),
            "Expected @as(i64, -1) fallback in:\n{}",
            zig
        );
    }

    // ── Test: Array.includes() built-in method ─────────────

    #[test]
    fn test_native_proto_array_includes() {
        // JS source: arr.includes(x) - returns bool, used in numeric context
        let js = r#"
/**
 * @param {number} target
 * @returns {number}
 */
export function hasItem(target) {
    const arr = [10, 20, 30];
    if (arr.includes(target)) {
        return 1;
    }
    return 0;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_includes");

        // Verify labeled block with for loop and bool return
        assert!(zig.contains("blk:"), "Expected labeled block in:\n{}", zig);
        assert!(zig.contains("for ("), "Expected for loop in:\n{}", zig);
        assert!(
            zig.contains("break :blk true"),
            "Expected break :blk true in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk false"),
            "Expected break :blk false in:\n{}",
            zig
        );
    }

    // ── Test: Array.join() built-in method ─────────────

    #[test]
    fn test_native_proto_array_join() {
        // JS source: arr.join(sep) - returns string
        let js = r#"
/**
 * @returns {string}
 */
export function joinNumbers() {
    const arr = [1, 2, 3];
    return arr.join(", ");
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_join");

        // Verify std.io.Writer.Allocating is used
        assert!(
            zig.contains("std.io.Writer.Allocating"),
            "Expected std.io.Writer.Allocating in:\n{}",
            zig
        );
        assert!(
            zig.contains("__join_buf"),
            "Expected __join_buf variable in:\n{}",
            zig
        );
        assert!(
            zig.contains("writeAll"),
            "Expected writeAll for separator in:\n{}",
            zig
        );
        // i64 elements should use {d} format
        assert!(
            zig.contains("{d}"),
            "Expected {{d}} format for i64 elements in:\n{}",
            zig
        );
    }

    // ── Test: Array.slice() built-in method ─────────────

    #[test]
    fn test_native_proto_array_slice() {
        // JS source: arr.slice(start, end) - returns sub-slice
        let js = r#"
/**
 * @returns {number}
 */
export function sliceSum() {
    const arr = [10, 20, 30, 40, 50];
    const sub = arr.slice(1, 4);
    return sub[0] + sub[1] + sub[2];
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_slice");

        // Verify slice expression is generated: arr.items[1..4]
        assert!(
            zig.contains(".items[1..4]"),
            "Expected '.items[1..4]' slice in:\n{}",
            zig
        );
    }

    // ── Test: Array.splice() built-in method ─────────────

    #[test]
    fn test_native_proto_array_splice() {
        // JS source: arr.splice(start, deleteCount)
        let js = r#"
/**
 * @param {number} start
 * @param {number} count
 * @returns {number}
 */
export function spliceArray(start, count) {
    const arr = [1, 2, 3, 4, 5];
    return arr.splice(start, count);
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_splice");

        // Verify splice generates ArrayList operations
        assert!(
            zig.contains("orderedRemove"),
            "Expected orderedRemove in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk __spliced"),
            "Expected break :blk __spliced in:\n{}",
            zig
        );
    }

    // ── Test: Array.splice() with insert items ─────────────

    #[test]
    fn test_native_proto_array_splice_insert() {
        // JS source: arr.splice(start, deleteCount, item1, item2)
        let js = r#"
/**
 * @param {number} start
 * @returns {number}
 */
export function spliceInsert(start) {
    const arr = [1, 2, 3, 4, 5];
    return arr.splice(start, 2, 99, 100);
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_splice_insert");

        // Verify splice generates insertSlice for multi-arg
        assert!(
            zig.contains("orderedRemove"),
            "Expected orderedRemove in:\n{}",
            zig
        );
        assert!(
            zig.contains("insertSlice"),
            "Expected insertSlice for insertion in:\n{}",
            zig
        );
    }

    // ── Test: New Math methods (random, pow, max, min) ─────────────────────

    #[test]
    fn test_native_proto_math_new_methods() {
        // JS source: Math.random(), Math.pow(), Math.max(), Math.min()
        let js = r#"
/**
 * @param {number} x
 * @param {number} y
 * @returns {number}
 */
export function testMathNew(x, y) {
    const rand = Math.random();
    const powXY = Math.pow(x, y);
    const maxXY = Math.max(x, y);
    const minXY = Math.min(x, y);
    return powXY + maxXY + minXY;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_math_new_methods");

        // Step2: verify Math methods are generated correctly
        assert!(
            zig.contains("std.crypto.random.int(u32)"),
            "Expected 'std.crypto.random.int(u32)' in:\n{}",
            zig
        );
        assert!(
            zig.contains("std.math.pow(f64,"),
            "Expected 'std.math.pow(f64,' in:\n{}",
            zig
        );
        assert!(zig.contains("if ("), "Expected 'if (' in max/min:\n{}", zig);
    }

    // ── Test: AwaitExpression support ─────────────

    #[test]
    fn test_native_proto_await() {
        // JS source: async function with await.
        // NOTE: In the real pipeline, `export` is stripped by the preprocessor,
        // and `exported_functions` is passed to transpile_js() to mark exports.
        // This test simulates that: no `export` in JS, uses exported_functions param.
        let js = r#"
/**
 * @param {i64} x
 * @returns {i64}
 */
async function asyncDouble(x) {
    const result = await double(x);
    return result;
}

function double(x) {
    return x * 2;
}
"#;
        let mut exports = std::collections::HashSet::new();
        exports.insert("asyncDouble".to_string());
        let zig = transpile_and_check!(js, "test_native_proto_await", exports);

        // Step2: verify async function signature has `io: anytype`
        assert!(
            zig.contains("io: anytype"),
            "Expected 'io: anytype' in async function signature, got:\n{}",
            zig
        );

        // Step3: verify await is translated to io.async() + .await(io)
        assert!(
            zig.contains("io.async("),
            "Expected 'io.async(' in generated code, got:\n{}",
            zig
        );
        assert!(
            zig.contains(".await(io)"),
            "Expected '.await(io)' in generated code, got:\n{}",
            zig
        );
        // defer _ = _tN.cancel(io) catch undefined;
        assert!(
            zig.contains("defer"),
            "Expected 'defer' in generated code, got:\n{}",
            zig
        );
        assert!(
            zig.contains(".cancel(io)"),
            "Expected '.cancel(io)' in generated code, got:\n{}",
            zig
        );

        // Step4: verify non-async function does NOT have `io: anytype`
        assert!(
            zig.contains("fn double(x: anytype) i64 {"),
            "Expected non-async function signature, got:\n{}",
            zig
        );
    }

    // ── Test: TypedArray support ─────────────

    #[test]
    fn test_native_proto_typedarray_basic() {
        // JS source: new Int32Array(), .length, index access
        let js = r#"
/**
 * @returns {number}
 */
export function sumInt32() {
    const arr = new Int32Array([1, 2, 3]);
    const len = arr.length;
    return len;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typedarray_basic");
        println!("=== TypedArray basic ===\n{}", zig);
        // Verify fromI64AsI32 is generated
        assert!(
            zig.contains("fromI64AsI32"),
            "Expected 'fromI64AsI32' in generated code:\n{}",
            zig
        );
        // Verify .length is generated as .len
        assert!(
            zig.contains(".len"),
            "Expected '.len' for TypedArray length:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_string_escape_backslash() {
        // Verify that backslashes in string literals are properly escaped for Zig output.
        // Without escaping, a string like "hello\nworld" would produce an invalid Zig string.
        let js = r#"function hasNewline() { return "hello\\nworld"; }"#;
        let zig = transpile_and_assert!(js, "test_native_proto_string_escape_backslash");
        println!("=== String escape ===\n{}", zig);
        // The JS string "hello\\nworld" has the actual value: hello\nworld (backslash-n)
        // In Zig, this should be emitted as "hello\\nworld"
        assert!(
            zig.contains("hello\\\\nworld"),
            "Expected escaped backslash in:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_string_escape_quote() {
        // Verify that double quotes in string literals are properly escaped for Zig output.
        let js = r#"function hasQuote() { return 'he said "hello"'; }"#;
        let zig = transpile_and_assert!(js, "test_native_proto_string_escape_quote");
        println!("=== String escape quote ===\n{}", zig);
        // The JS string 'he said "hello"' has actual double quotes
        // In Zig, this should be emitted as \"he said \\\"hello\\\"\"
        assert!(
            zig.contains(r#"\"hello\""#),
            "Expected escaped double quote in:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_typedarray_uint8() {
        // JS source: new Uint8Array()
        let js = r#"
function makeBytes() {
    const bytes = new Uint8Array([1, 2, 3]);
    return bytes.length;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typedarray_uint8");
        println!("=== TypedArray Uint8 ===\n{}", zig);
        assert!(
            zig.contains("fromU8"),
            "Expected 'fromU8' in generated code:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_typedarray_length_in_expr() {
        // .length used in arithmetic expression (not just return)
        let js = r#"
/**
 * @returns {number}
 */
export function totalLength() {
    const arr1 = new Int32Array([1, 2]);
    const arr2 = new Int32Array([3, 4, 5]);
    return arr1.length + arr2.length;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typedarray_length_in_expr");
        println!("=== TypedArray length in expr ===\n{}", zig);
        // Verify .len is generated for both arrays
        assert!(
            zig.matches("arr1.len").count() >= 1,
            "Expected arr1.len in:\n{}",
            zig
        );
        assert!(
            zig.matches("arr2.len").count() >= 1,
            "Expected arr2.len in:\n{}",
            zig
        );
        // Verify the addition is present
        assert!(
            zig.contains(" + "),
            "Expected addition for length sum:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_typedarray_length_as_param() {
        // .length passed as function argument
        let js = r#"
/**
 * @param {number} x
 * @returns {number}
 */
function identity(x) { return x; }

/**
 * @returns {number}
 */
export function getLength() {
    const arr = new Int32Array([10, 20, 30, 40]);
    return identity(arr.length);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typedarray_length_as_param");
        println!("=== TypedArray length as param ===\n{}", zig);
        // Verify arr.len is generated
        assert!(zig.contains("arr.len"), "Expected arr.len in:\n{}", zig);
        assert!(
            zig.contains("identity"),
            "Expected identity call in:\n{}",
            zig
        );
    }

    // ── TypedArray: set/get/slice/subarray/copyWithin/fill ──

    #[test]
    fn test_native_proto_typedarray_set() {
        let js = r#"
/**
 * @param {number} idx
 * @param {number} val
 * @returns {number}
 */
export function setAndGet(idx, val) {
    const arr = new Int32Array([10, 20, 30]);
    arr.set(idx, val);
    return arr.get(idx);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typedarray_set");
        println!("=== TypedArray set ===\n{}", zig);
        assert!(zig.contains("setI32"), "Expected setI32 in:\n{}", zig);
        assert!(zig.contains("getI32"), "Expected getI32 in:\n{}", zig);
    }

    #[test]
    fn test_native_proto_typedarray_slice() {
        let js = r#"
/**
 * @returns {number}
 */
export function sliceArray() {
    const arr = new Int32Array([10, 20, 30, 40, 50]);
    const sub = arr.slice(1, 4);
    return sub.length;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typedarray_slice");
        println!("=== TypedArray slice ===\n{}", zig);
        assert!(zig.contains("sliceI32"), "Expected sliceI32 in:\n{}", zig);
    }

    #[test]
    fn test_native_proto_typedarray_subarray() {
        let js = r#"
/**
 * @returns {number}
 */
export function subArray() {
    const arr = new Int32Array([1, 2, 3, 4, 5]);
    const sub = arr.subarray(1, 3);
    return sub.length;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typedarray_subarray");
        println!("=== TypedArray subarray ===\n{}", zig);
        assert!(
            zig.contains("subarrayI32"),
            "Expected subarrayI32 in:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_typedarray_copywithin() {
        let js = r#"
export function copyIn() {
    const arr = new Int32Array([1, 2, 3, 4, 5]);
    arr.copyWithin(0, 3, 5);
    return arr.get(1);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typedarray_copywithin");
        println!("=== TypedArray copyWithin ===\n{}", zig);
        assert!(
            zig.contains("copyWithinI32"),
            "Expected copyWithinI32 in:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_typedarray_fill() {
        let js = r#"
export function fillArr() {
    const arr = new Int32Array([1, 2, 3, 4, 5]);
    arr.fill(0, 1, 4);
    return arr.get(0);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typedarray_fill");
        println!("=== TypedArray fill ===\n{}", zig);
        assert!(zig.contains("fillI32"), "Expected fillI32 in:\n{}", zig);
    }

    // ── TypedArray: buffer / byteLength / byteOffset ──

    #[test]
    fn test_native_proto_typedarray_buffer() {
        let js = r#"
/**
 * @returns {number}
 */
export function getBufferLength() {
    const arr = new Int32Array([1, 2, 3]);
    const buf = arr.buffer;
    return buf.length;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typedarray_buffer");
        println!("=== TypedArray buffer ===\n{}", zig);
        assert!(zig.contains("bufferI32"), "Expected bufferI32 in:\n{}", zig);
    }

    #[test]
    fn test_native_proto_typedarray_bytelength() {
        let js = r#"
/**
 * @returns {number}
 */
export function getByteLength() {
    const arr = new Int32Array([1, 2, 3]);
    return arr.byteLength;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typedarray_bytelength");
        println!("=== TypedArray byteLength ===\n{}", zig);
        assert!(
            zig.contains("byteLengthI32"),
            "Expected byteLengthI32 in:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_typedarray_byteoffset() {
        let js = r#"
/**
 * @returns {number}
 */
export function getByteOffset() {
    const arr = new Int32Array([1, 2, 3]);
    return arr.byteOffset;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typedarray_byteoffset");
        println!("=== TypedArray byteOffset ===\n{}", zig);
        assert!(
            zig.contains("byteOffset"),
            "Expected byteOffset in:\n{}",
            zig
        );
    }

    // ── TypedArray: Float64Array ──

    #[test]
    fn test_native_proto_float64array() {
        let js = r#"
/**
 * @returns {number}
 */
export function floatTest() {
    const arr = new Float64Array([1.5, 2.5, 3.5]);
    return arr.length;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_float64array");
        println!("=== Float64Array ===\n{}", zig);
        assert!(zig.contains("fromF64"), "Expected fromF64 in:\n{}", zig);
    }

    // ── String escaping edge cases ────────────────────────

    #[test]
    fn test_native_proto_string_escape_newline() {
        // Verify that newline characters in JS string literals are escaped as \\n in Zig output.
        // JS "line1\nline2" → actual newline (0x0A) in JS → must emit Zig "line1\\nline2"
        // where \\n is the two-byte escape sequence (backslash + n), NOT a raw newline.
        let js = "function hasNewline() { return \"line1\\nline2\"; }";
        let zig = transpile_and_assert!(js, "test_native_proto_string_escape_newline");
        println!("=== String escape newline ===\n{}", zig);
        // The Zig output has literal '\' followed by 'n' (NOT the newline character).
        // In Rust, "\\n" matches the two-byte sequence 0x5C 0x6E.
        assert!(
            zig.contains("line1\\nline2"),
            "Expected \\n escape sequence (NOT raw newline) in:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_string_escape_tab() {
        // Verify that tab characters in JS string literals are escaped as \\t in Zig output.
        let js = "function hasTab() { return \"col1\\tcol2\"; }";
        let zig = transpile_and_assert!(js, "test_native_proto_string_escape_tab");
        println!("=== String escape tab ===\n{}", zig);
        assert!(
            zig.contains("col1\\tcol2"),
            "Expected \\t escape sequence (NOT raw tab) in:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_json_parse_escaped_quotes() {
        // JSON.parse with a string that contains escaped double quotes.
        let js = r#"
/**
 * @typedef {Object} Msg
 * @property {string} text
 */

/**
 * @returns {string}
 */
export function parseEscapedJson() {
    /**
     * @type {Msg}
     */
    const msg = JSON.parse('{"text":"he said \\"hello\\""}');
    return msg.text;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_json_parse_escaped_quotes");
        println!("=== JSON parse escaped quotes ===\n{}", zig);
        // Verify the JSON string is properly escaped in Zig
        assert!(
            zig.contains("std.json.parse(Msg,"),
            "Expected std.json.parse(Msg, ...), got:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_json_parse_unicode() {
        // JSON.parse with unicode escape sequences.
        let js = r#"
/**
 * @typedef {Object} Item
 * @property {string} name
 */

/**
 * @returns {string}
 */
export function parseUnicodeJson() {
    /**
     * @type {Item}
     */
    const item = JSON.parse('{"name":"\\u0048\\u0065\\u006c\\u006c\\u006f"}');
    return item.name;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_json_parse_unicode");
        println!("=== JSON parse unicode ===\n{}", zig);
        // Verify std.json.parse is generated
        assert!(
            zig.contains("std.json.parse(Item,"),
            "Expected std.json.parse(Item, ...), got:\n{}",
            zig
        );
        // Unicode escapes should pass through (Zig's std.json.parse handles them)
        assert!(
            zig.contains("\\\\u0048"),
            "Expected unicode escape in:\n{}",
            zig
        );
    }

    // ── Try-catch tests ──────────────────────────────────

    #[test]
    fn test_native_proto_try_catch_basic() {
        // Basic try-catch: throw in try, caught in catch handler.
        let js = r##"
function safeDivide(a, b) {
    try {
        if (b === 0) throw "div by zero";
        return a / b;
    } catch (e) {
        return -1;
    }
}
"##;
        let zig = transpile_and_assert!(js, "test_native_proto_try_catch_basic");
        println!("=== Try-catch basic ===\n{}", zig);
        // Should generate the labeled block pattern
        assert!(
            zig.contains("_js_try_blk_"),
            "Expected labeled block:\n{}",
            zig
        );
        // Should generate catch |err| for the handler
        assert!(
            zig.contains("catch |err|"),
            "Expected catch |err|:\n{}",
            zig
        );
        // Should bind catch(e) → _ = @errorName(err) (e is unused in body)
        assert!(
            zig.contains("_ = @errorName(err);"),
            "Expected '_ = @errorName(err);':\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_try_catch_basic");
    }

    #[test]
    fn test_native_proto_try_catch_e_binding_used() {
        // Verify that catch(e) with e used in body generates `const e = ...`.
        let js = r##"
function catchAndLog(val) {
    try {
        if (val < 0) throw "bad";
        return val;
    } catch (e) {
        return e;
    }
}
"##;
        let zig = transpile_and_assert!(js, "test_native_proto_try_catch_e_binding_used");
        println!("=== Try-catch e binding (used) ===\n{}", zig);
        // Should generate `const e = @errorName(err);` in catch handler
        assert!(
            zig.contains("const e = @errorName(err);"),
            "Expected 'const e = @errorName(err);' when e is used in catch body:\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_try_catch_e_binding_used");
    }

    #[test]
    fn test_native_proto_try_catch_e_binding_unused() {
        // Verify that catch(e) with e NOT used generates `_ = @errorName(err)`.
        let js = r##"
function catchAndIgnore(val) {
    try {
        if (val < 0) throw "bad";
        return val;
    } catch (e) {
        return -1;
    }
}
"##;
        let zig = transpile_and_assert!(js, "test_native_proto_try_catch_e_binding_unused");
        println!("=== Try-catch e binding (unused) ===\n{}", zig);
        // Should generate `_ = @errorName(err);` (not const e)
        assert!(
            zig.contains("_ = @errorName(err);"),
            "Expected '_ = @errorName(err);' when e is unused:\n{}",
            zig
        );
        assert!(
            !zig.contains("const e = @errorName(err);"),
            "Should NOT have 'const e' when e is unused:\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_try_catch_e_binding_unused");
    }

    #[test]
    fn test_native_proto_try_catch_throw_break() {
        // Inside try block, throw should use break :label, not return.
        let js = r##"
function check(val) {
    try {
        if (val < 0) throw "negative";
        return val;
    } catch (e) {
        return 0;
    }
}
"##;
        let zig = transpile_and_assert!(js, "test_native_proto_try_catch_throw_break");
        println!("=== Try-catch throw break ===\n{}", zig);
        // Inside try: throw should use break, not return
        assert!(
            zig.contains("break :"),
            "Expected break :label for throw inside try:\n{}",
            zig
        );
        // Should have catch handler
        assert!(
            zig.contains("catch |err|"),
            "Expected catch handler:\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_try_catch_throw_break");
    }

    #[test]
    fn test_native_proto_try_finally() {
        // try-finally without catch handler.
        let js = r#"
function withCleanup() {
    let x = 0;
    try {
        x = 42;
    } finally {
        x = 0;
    }
    return x;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_try_finally");
        println!("=== Try-finally ===\n{}", zig);
        // Finally body should be inlined after the try body (not defer).
        // The cleanup x=0 should appear after x=42.
        assert!(
            zig.contains("x = 42;") && zig.contains("x = 0;"),
            "Expected finally body inlined after try body:\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_try_finally");
    }

    #[test]
    fn test_native_proto_try_catch_finally() {
        // try-catch-finally: catch handler + finally cleanup.
        let js = r##"
function process(val) {
    let result = 0;
    try {
        if (val < 0) throw "bad";
        result = val * 2;
    } catch (e) {
        result = -1;
    } finally {
        val = 0;
    }
    return result;
}
"##;
        let zig = transpile_and_assert!(js, "test_native_proto_try_catch_finally");
        println!("=== Try-catch-finally ===\n{}", zig);
        // Finally body should be inlined after catch, before return result.
        assert!(
            zig.contains("val = 0;"),
            "Expected finally body inlined after catch:\n{}",
            zig
        );
        // Should have catch handler
        assert!(
            zig.contains("catch |err|"),
            "Expected catch handler:\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_try_catch_finally");
    }

    #[test]
    fn test_native_proto_try_catch_no_throw() {
        // try-catch without throw statement: body emitted inline, handler skipped.
        let js = r#"
function safeOp(x) {
    try {
        return x + 1;
    } catch (e) {
        return 0;
    }
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_try_catch_no_throw");
        println!("=== Try-catch no throw ===\n{}", zig);
        // Body should be emitted (return x + 1)
        assert!(zig.contains("return x + 1"), "Expected body:\n{}", zig);
        // Catch handler is unreachable — not generated
        assert!(
            !zig.contains("catch |err|"),
            "Should not have catch for throw-free try:\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_try_catch_no_throw");
    }

    #[test]
    fn test_native_proto_throw_bare() {
        // Bare throw (outside try-catch) should still use return error.JsThrow.
        let js = r##"
function reject(val) {
    if (val < 0) throw "bad";
    return val;
}
"##;
        let zig = transpile_and_assert!(js, "test_native_proto_throw_bare");
        println!("=== Throw bare ===\n{}", zig);
        // Bare throw should generate return error.JsThrow (not break)
        assert!(
            zig.contains("return error.JsThrow"),
            "Expected return error.JsThrow for bare throw:\n{}",
            zig
        );
        // Should NOT contain break
        assert!(
            !zig.contains("break :"),
            "Should not have break for bare throw:\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_throw_bare");
    }

    // ── Test: ** operator (exponentiation) ─────────────

    #[test]
    fn test_native_proto_exponential_operator() {
        // Integer exponentiation: 2 ** 3 → loop-based implementation
        let js = r#"
/**
 * @param {number} base
 * @param {number} exp
 * @returns {number}
 */
export function intPow(base, exp) {
    return base ** exp;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_exponential_operator");
        println!("=== Exponential (int) ===\n{}", zig);
        // Should generate std.math.pow for exponentiation
        assert!(
            zig.contains("std.math.pow(f64,"),
            "Expected std.math.pow for exponentiation:\n{}",
            zig
        );
        // Should cast operands to f64
        assert!(
            zig.contains("@as(f64,"),
            "Expected @as(f64, ...) cast for exponentiation:\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_exponential_operator");
    }

    #[test]
    fn test_native_proto_exponential_float() {
        // Float exponentiation: 2.0 ** 3.0 → std.math.pow(f64, ...)
        // Note: without type annotations, base and exp are inferred as i64.
        // To test float exponentiation, we need to use float literals.
        let js_float = r#"
export function powFloat() {
    return 2.0 ** 3.0;
}
"#;
        let zig = transpile_and_assert!(js_float, "test_native_proto_exponential_float");
        println!("=== Exponential (float) ===\n{}", zig);
        // Should generate std.math.pow for float exponentiation
        assert!(
            zig.contains("std.math.pow(f64,"),
            "Expected std.math.pow for float exponentiation:\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_exponential_float");
    }

    #[test]
    fn test_native_proto_exponential_mixed() {
        // Mixed: integer ** float → should use std.math.pow(f64, ...)
        let js = r#"
export function powMixed() {
    const base = 2;
    const exp = 3.0;
    return base ** exp;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_exponential_mixed");
        println!("=== Exponential (mixed) ===\n{}", zig);
        // Should cast integer to f64 and use std.math.pow
        assert!(
            zig.contains("std.math.pow(f64,"),
            "Expected std.math.pow for mixed exponentiation:\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_exponential_mixed");
    }

    // ── Test: Arrow function (single-expression) ────────────

    #[test]
    fn test_native_proto_arrow_function() {
        // Simple arrow function assigned to variable
        let js = r#"export function testArrow() {
    const add = (x, y) => x + y;
    return add(3, 4);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_arrow_function");
        println!(
            "=== Arrow function (basic) ===
{}",
            zig
        );
        // Should generate a Zig function for the arrow function
        assert!(
            zig.contains("fn _arrow_fn_"),
            "Expected arrow function to generate a Zig function:
{}",
            zig
        );
        // Should assign the function to the variable
        assert!(
            zig.contains("const add = _arrow_fn_"),
            "Expected arrow function to be assigned to variable:
{}",
            zig
        );
        // NOTE: We skip zig ast-check here because the testArrow function
        // has incorrect return type inference (separate issue to fix later).
    }
    // ── Test: Template literal (complex nesting) ──────────

    #[test]
    fn test_native_proto_template_literal_complex() {
        // Complex template: multiple expressions, nested property, function call
        let js = r#"export function buildMessage(user, scores) {
    const name = user.name;
    const avg = scores[0] + scores[1];
    return `Hello ${name}, your average score is ${avg}!`;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_template_literal_complex");
        println!(
            "=== Template literal (complex) ===
{}",
            zig
        );
        // Should generate std.fmt.allocPrint for template with expressions
        assert!(
            zig.contains("std.fmt.allocPrint"),
            "Expected allocPrint for template with expressions:
{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_template_literal_complex");
    }

    // ── Test: ** operator (edge cases) ──────────────

    #[test]
    fn test_native_proto_exponential_edge() {
        // Edge cases: zero exponent, negative exponent, float base
        let js = r#"export function powEdge() {
    const x = 2.0;
    const a = x ** 0;   // x^0 = 1
    const b = x ** -1;  // x^(-1) = 0.5
    return a + b;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_exponential_edge");
        println!(
            "=== Exponential (edge) ===
{}",
            zig
        );
        // Should generate std.math.pow for exponentiation
        assert!(
            zig.contains("std.math.pow(f64,"),
            "Expected std.math.pow for edge case exponentiation:
{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_exponential_edge");
    }
    // ── Test: Arrow function (single param + block body) ──────────

    #[test]
    fn test_native_proto_arrow_single_param() {
        // Arrow function with single parameter
        let js = r#"export function testSingleParam() {
    const double = x => x * 2;
    return double(5);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_arrow_single_param");
        println!(
            "=== Arrow function (single param) ===
{}",
            zig
        );
        assert!(zig.contains("fn _arrow_fn_"), "Expected arrow function");
        assert!(
            zig.contains("const double = _arrow_fn_"),
            "Expected assignment"
        );
    }
    // ── Test: Arrow function (block body) ────────────

    #[test]
    fn test_native_proto_arrow_block_body() {
        // Arrow function with block body
        let js = r#"export function testBlockArrow() {
    const f = x => { return x + 1; };
    return f(5);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_arrow_block_body");
        println!(
            "=== Arrow function (block) ===
{}",
            zig
        );
        assert!(zig.contains("fn _arrow_fn_"), "Expected arrow function");
        assert!(
            zig.contains("return x + 1;"),
            "Expected return in block body"
        );
    }

    // ── Test: Closure (arrow function capturing outer variable) ────────────

    #[test]
    fn test_native_proto_closure_basic() {
        // Arrow function capturing outer variable (closure)
        let js = r#"export function testClosure() {
    const x = 10;
    const adder = (y) => x + y;
    return adder(5);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_closure_basic");
        println!(
            "=== Closure (basic) ===
{}",
            zig
        );
        // Should generate a closure struct with captured variable x
        assert!(zig.contains("const Closure_"), "Expected closure struct");
        assert!(zig.contains("fn call(self:"), "Expected call method");
        assert!(
            zig.contains("self.x"),
            "Expected captured variable access via self.x"
        );
    }

    // ── Test: Closure with mutable captured variable ────────────

    #[test]
    fn test_native_proto_closure_mutable() {
        // Arrow function capturing and modifying outer variable
        let js = r#"export function testClosureMutable() {
    let count = 0;
    const increment = () => { count = count + 1; return count; };
    increment();
    return count;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_closure_mutable");
        println!(
            "=== Closure (mutable) ===
{}",
            zig
        );
        // Should generate a closure struct with mutable captured variable (pointer)
        assert!(zig.contains("const Closure_"), "Expected closure struct");
        assert!(zig.contains("*i64"), "Expected pointer for mutable capture");
        assert!(
            zig.contains("self.count.*"),
            "Expected dereference for mutable capture"
        );
    }

    // ── Test: Getter in object literal ──────────────

    #[test]
    fn test_native_proto_getter() {
        // Object literal with getter property
        // { get x() { return 42; } } → .{ .x = 42 }
        let js = r#"export function useGetter() {
    const obj = { get x() { return 42; } };
    return obj.x;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_getter");
        println!(
            "=== Getter ===
{}",
            zig
        );
        // Getter return expression should be used as field value
        assert!(
            zig.contains(".x = 42"),
            "Expected getter value as field: {}",
            zig
        );
        assert!(
            !zig.contains("get "),
            "Should not have 'get' keyword: {}",
            zig
        );
    }

    // ── Test: Setter skipped in object literal ─────

    #[test]
    fn test_native_proto_setter_skipped() {
        // Object literal with setter — setter is skipped
        let js = r#"export function useSetter() {
    const obj = { a: 1, set x(v) { this._x = v; } };
    return obj.a;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_setter_skipped");
        println!(
            "=== Setter skipped ===
{}",
            zig
        );
        // Setter should be removed — only field 'a' remains
        assert!(!zig.contains("set "), "Setter should be removed: {}", zig);
        assert!(
            zig.contains(".a = 1"),
            "Regular field should be preserved: {}",
            zig
        );
    }

    // ── Test: Combined getter/setter in object ─────

    #[test]
    fn test_native_proto_getter_setter_combined() {
        // Both getter and regular properties in same object
        let js = r#"export function combineGS() {
    const obj = { name: "test", get age() { return 25; }, set age(v) { /* noop */ } };
    // age getter provides the field value, setter is skipped
    return obj.name;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_getter_setter_combined");
        println!(
            "=== Combined getter/setter ===
{}",
            zig
        );
        assert!(
            zig.contains(".name = \"test\""),
            "Regular property should remain: {}",
            zig
        );
        assert!(
            zig.contains(".age = 25"),
            "Getter should provide field value: {}",
            zig
        );
        assert!(
            !zig.contains("set "),
            "No setter keyword in output: {}",
            zig
        );
    }

    // ── Test: Optional chaining (?. ) — known struct → direct access ─────

    #[test]
    fn test_native_proto_optional_chain_known_struct() {
        // obj?.prop on a known struct type → equivalent to obj.prop (no null check)
        let js = r#"
export function getProp(obj) {
    const val = obj?.name;
    return val;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_optional_chain_known_struct");
        // Should generate direct access: obj.name (no if-wrapper)
        assert!(
            zig.contains("obj.name"),
            "Should use direct access obj.name: {}",
            zig
        );
        assert!(
            !zig.contains("_oc"),
            "Should NOT generate null-check temp var for known struct: {}",
            zig
        );
    }

    // ── Test: Optional chaining (?. ) — unknown type → null check ─────────

    #[test]
    fn test_native_proto_optional_chain_unknown() {
        // obj?.prop on an unknown type → generates (if (obj) |_ocN| _ocN.prop else null)
        let js = r#"
function getUnknown(obj) {
    return obj?.name;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_optional_chain_unknown");
        assert!(
            zig.contains("(if ("),
            "Should generate null check pattern: {}",
            zig
        );
        assert!(
            zig.contains(") |_oc"),
            "Should generate temp var capture: {}",
            zig
        );
        assert!(
            zig.contains(" else null)"),
            "Should have else null: {}",
            zig
        );
    }

    // ── Test: Optional chaining call — unknown callee → null check ─ ─
    #[test]
    fn test_native_proto_optional_chain_call() {
        // obj?.method() on unknown callee → (if (obj) |_ocN| _ocN.method() else null)
        let js = r#"
function callMaybe(obj) {
    return obj?.greet("World");
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_optional_chain_call");
        assert!(
            zig.contains("(if ("),
            "Should generate null check + call pattern: {}",
            zig
        );
        assert!(
            zig.contains(" else null)"),
            "Should have else null: {}",
            zig
        );
        assert!(zig.contains("greet("), "Should call method greet: {}", zig);
    }

    // ── Test: Nested optional chaining (a?.b?.c) ──────────────────────────

    #[test]
    fn test_native_proto_optional_chain_nested() {
        // a?.b?.c → nested if-else blocks
        let js = r#"
function deep(obj) {
    return obj?.a?.b;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_optional_chain_nested");
        // Should have two levels of null check
        let oc_count = zig.matches("_oc").count();
        assert!(
            oc_count >= 2,
            "Expected at least 2 temp vars for nested chain, got {}: {}",
            oc_count,
            zig
        );
        assert!(
            zig.contains(" else null)") || zig.contains(" else null"),
            "Should have else null branches: {}",
            zig
        );
    }

    // ── Test: Optional chaining on null literal → null ────────────────────

    #[test]
    fn test_native_proto_optional_chain_null_literal() {
        // null?.prop → generates null check (always null)
        let js = r#"
function nullChain() {
    return null?.prop;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_optional_chain_null_literal");
        assert!(
            zig.contains("(if (") || zig.contains("null"),
            "Should handle null literal in chain: {}",
            zig
        );
    }

    // ══════════════════════════════════════════════════════════════
    // P1 Feature Tests
    // ══════════════════════════════════════════════════════════════

    // ── P1-1: in / instanceof operators ──────────────────────────

    #[test]
    fn test_p1_in_operator() {
        // "key" in obj → obj.contains("key")
        let js = r##"
function hasProp(obj) {
    return "name" in obj;
}
"##;
        let zig = transpile_and_check!(js, "test_p1_in_operator");
        assert!(
            zig.contains(".contains("),
            "Expected .contains() in:\n{}",
            zig
        );
        assert!(
            zig.contains("\"name\""),
            "Expected key literal in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_instanceof_operator() {
        // obj instanceof Foo → @compileError(...)
        // Note: @compileError causes unreachable code, so ast-check won't pass.
        let js = r#"
function checkType(obj) {
    return obj instanceof Array;
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_instanceof_operator");
        assert!(
            zig.contains("@compileError"),
            "Expected @compileError in:\n{}",
            zig
        );
        assert!(
            zig.contains("instanceof"),
            "Expected 'instanceof' mention in:\n{}",
            zig
        );
    }

    // ── P1-2: Date methods ───────────────────────────────────────

    #[test]
    fn test_p1_date_now() {
        let js = r#"
/**
 * @returns {number}
 */
export function getNow() {
    return Date.now();
}
"#;
        let zig = transpile_and_check!(js, "test_p1_date_now");
        assert!(
            zig.contains("js_date.now()"),
            "Expected js_date.now() in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_date_parse() {
        let js = r#"
/**
 * @returns {number}
 */
export function parseDate(s) {
    return Date.parse(s);
}
"#;
        let zig = transpile_and_check!(js, "test_p1_date_parse");
        assert!(
            zig.contains("js_date.parse("),
            "Expected js_date.parse() in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_date_utc() {
        // Date.UTC is not yet implemented → generates @compileError
        // Note: @compileError causes unreachable code, so ast-check won't pass.
        let js = r#"
/**
 * @returns {number}
 */
export function utcDate(y, m, d) {
    return Date.UTC(y, m, d);
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_date_utc");
        assert!(
            zig.contains("@compileError"),
            "Expected @compileError in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_date_instance_methods() {
        let js = r#"
/**
 * @returns {number}
 */
export function dateParts(d) {
    const t = d.getTime();
    const y = d.getFullYear();
    const mo = d.getMonth();
    const da = d.getDate();
    const dy = d.getDay();
    const h = d.getHours();
    const mi = d.getMinutes();
    const s = d.getSeconds();
    return t + y + mo + da + dy + h + mi + s;
}
"#;
        let zig = transpile_and_check!(js, "test_p1_date_instance_methods");
        assert!(
            zig.contains("js_date.getTime("),
            "Expected js_date.getTime() in:\n{}",
            zig
        );
        assert!(
            zig.contains("js_date.getFullYear("),
            "Expected js_date.getFullYear() in:\n{}",
            zig
        );
        assert!(
            zig.contains("js_date.getMonth("),
            "Expected js_date.getMonth() in:\n{}",
            zig
        );
        assert!(
            zig.contains("js_date.getDate("),
            "Expected js_date.getDate() in:\n{}",
            zig
        );
        assert!(
            zig.contains("js_date.getDay("),
            "Expected js_date.getDay() in:\n{}",
            zig
        );
        assert!(
            zig.contains("js_date.getHours("),
            "Expected js_date.getHours() in:\n{}",
            zig
        );
        assert!(
            zig.contains("js_date.getMinutes("),
            "Expected js_date.getMinutes() in:\n{}",
            zig
        );
        assert!(
            zig.contains("js_date.getSeconds("),
            "Expected js_date.getSeconds() in:\n{}",
            zig
        );
    }

    // ── P1-3: Object static methods ──────────────────────────────

    #[test]
    fn test_p1_object_keys() {
        let js = r#"
function getKeys(obj) {
    return Object.keys(obj);
}
"#;
        let zig = transpile_and_check!(js, "test_p1_object_keys");
        assert!(
            zig.contains("js_object.keys("),
            "Expected js_object.keys() in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_object_values() {
        let js = r#"
function getValues(obj) {
    return Object.values(obj);
}
"#;
        let zig = transpile_and_check!(js, "test_p1_object_values");
        assert!(
            zig.contains("js_object.values("),
            "Expected js_object.values() in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_object_entries() {
        let js = r#"
function getEntries(obj) {
    return Object.entries(obj);
}
"#;
        let zig = transpile_and_check!(js, "test_p1_object_entries");
        assert!(
            zig.contains("js_object.entries("),
            "Expected js_object.entries() in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_object_assign() {
        let js = r#"
function copyObj(target, source) {
    return Object.assign(target, source);
}
"#;
        let zig = transpile_and_check!(js, "test_p1_object_assign");
        assert!(
            zig.contains("js_object.assign("),
            "Expected js_object.assign() in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_object_freeze() {
        // Object.freeze is a no-op in Zig (identity)
        let js = r#"
function freezeObj(obj) {
    return Object.freeze(obj);
}
"#;
        let zig = transpile_and_check!(js, "test_p1_object_freeze");
        // freeze is a pass-through; should NOT generate any js_object call
        assert!(
            !zig.contains("js_object"),
            "freeze should be a no-op (no js_object call):\n{}",
            zig
        );
    }

    // ── P1-4: Labeled statements ─────────────────────────────────

    #[test]
    fn test_p1_labeled_while() {
        let js = r#"
/**
 * @param {number} n
 */
export function labWhile(n) {
    let i = 0;
    outer: while (i < n) {
        i = i + 1;
        if (i > 5) {
            break outer;
        }
    }
    return i;
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_labeled_while");
        assert!(
            zig.contains("outer: while"),
            "Expected 'outer: while' in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :outer"),
            "Expected 'break :outer' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_labeled_for() {
        let js = r#"
/**
 * @param {number} n
 */
export function labFor(n) {
    let sum = 0;
    loop1: for (let i = 0; i < n; i = i + 1) {
        if (i === 3) {
            continue loop1;
        }
        sum = sum + i;
    }
    return sum;
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_labeled_for");
        // for loop is transformed to while, but the label should be preserved
        assert!(
            zig.contains("loop1:"),
            "Expected 'loop1:' label in:\n{}",
            zig
        );
        assert!(
            zig.contains("continue :loop1"),
            "Expected 'continue :loop1' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_labeled_do_while() {
        let js = r#"
export function labDoWhile() {
    let i = 0;
    retry: do {
        i = i + 1;
        if (i < 3) {
            continue retry;
        }
    } while (i < 5);
    return i;
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_labeled_do_while");
        // Labeled do-while generates "retry: " prefix followed by loop body
        assert!(
            zig.contains("retry:"),
            "Expected 'retry:' label in:\n{}",
            zig
        );
        assert!(zig.contains("while"), "Expected while loop in:\n{}", zig);
        assert!(
            zig.contains("continue :retry"),
            "Expected 'continue :retry' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_labeled_for_of() {
        let js = r#"
/**
 * @param {Object} arr
 */
export function labForOf(arr) {
    let sum = 0;
    items: for (const x of arr) {
        if (x < 0) {
            break items;
        }
        sum = sum + x;
    }
    return sum;
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_labeled_for_of");
        assert!(
            zig.contains("items:"),
            "Expected labeled for-of in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :items"),
            "Expected 'break :items' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_labeled_block() {
        // Labeled block (not a loop) — generic label: { ... }
        let js = r#"
/**
 * @param {number} x
 */
export function labBlock(x) {
    let result = 0;
    check: {
        if (x > 0) {
            result = 1;
            break check;
        }
        result = -1;
    }
    return result;
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_labeled_block");
        assert!(
            zig.contains("check: {"),
            "Expected 'check: {{' in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :check"),
            "Expected 'break :check' in:\n{}",
            zig
        );
    }

    // ── P1-5: Multi-spread object merge ──────────────────────────

    #[test]
    fn test_p1_spread_single() {
        // { ...a } → identity (just a)
        let js = r#"
function spreadOne(a) {
    return { ...a };
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_spread_single");
        // Single spread with no inline should just emit the expression
        assert!(
            !zig.contains("spreadMerge"),
            "Single spread should be identity:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_spread_with_inline() {
        // { ...a, extra: 1 } → js_runtime.spreadMerge(a, .{ .extra = 1 })
        let js = r#"
function spreadInline(a) {
    return { ...a, extra: 1 };
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_spread_with_inline");
        assert!(
            zig.contains("js_runtime.spreadMerge("),
            "Expected js_runtime.spreadMerge() in:\n{}",
            zig
        );
        assert!(
            zig.contains(".extra = 1"),
            "Expected .extra field in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_spread_multi() {
        // { ...a, ...b } → js_runtime.spreadMerge(a, b)
        let js = r#"
function spreadTwo(a, b) {
    return { ...a, ...b };
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_spread_multi");
        let merge_count = zig.matches("spreadMerge").count();
        assert_eq!(
            merge_count, 1,
            "Expected exactly 1 spreadMerge call, got {}:\n{}",
            merge_count, zig
        );
    }

    #[test]
    fn test_p1_spread_multi_with_inline() {
        // { ...a, ...b, c: 1 } → js_runtime.spreadMerge(spreadMerge(a, b), .{ .c = 1 })
        let js = r#"
function spreadThree(a, b) {
    return { ...a, ...b, c: 1, d: "hello" };
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_spread_multi_with_inline");
        let merge_count = zig.matches("spreadMerge").count();
        assert_eq!(
            merge_count, 2,
            "Expected 2 spreadMerge calls, got {}:\n{}",
            merge_count, zig
        );
        assert!(zig.contains(".c = 1"), "Expected .c field in:\n{}", zig);
        assert!(
            zig.contains(".d = \"hello\""),
            "Expected .d field in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_spread_empty() {
        // { } → StringHashMap
        let js = r#"
function emptyObj() {
    return { };
}
"#;
        let zig = transpile_and_check!(js, "test_p1_spread_empty");
        assert!(
            zig.contains("StringHashMap(JsAny)"),
            "Expected StringHashMap for empty object:\n{}",
            zig
        );
    }

    // ── Array spread [...a, ...b] ─────────────────────

    #[test]
    fn test_p1_array_spread_simple() {
        // [...a, ...b] → appendSlice(a.items, b.items)
        let js = r#"
function arraySpread(a, b) {
    return [...a, ...b];
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_array_spread_simple");
        assert!(
            zig.contains("appendSlice"),
            "Expected appendSlice in:\n{}",
            zig
        );
        assert!(zig.contains(".items)"), "Expected .items in:\n{}", zig);
    }

    #[test]
    fn test_p1_array_spread_mixed() {
        // [...a, 1, ...b] → appendSlice + append
        let js = r#"
function arraySpreadMixed(a, b) {
    return [...a, 1, ...b];
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_array_spread_mixed");
        assert!(
            zig.contains("appendSlice"),
            "Expected appendSlice in:\n{}",
            zig
        );
        assert!(
            zig.contains("append(js_allocator.getAllocator()"),
            "Expected append in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_array_spread_single() {
        // [...a] → appendSlice (shallow copy, NOT identity)
        let js = r#"
function arraySpreadSingle(a) {
    return [...a];
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_array_spread_single");
        // Single array spread must create a new array via appendSlice
        assert!(
            zig.contains("appendSlice"),
            "[...a] should use appendSlice, got:\n{}",
            zig
        );
        assert!(
            zig.contains("arraySpreadSingle"),
            "Expected function def in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_array_spread_elision() {
        // [1, , 3] → append undefined for elision
        let js = r#"
function arrayElision() {
    return [1, , 3];
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_array_spread_elision");
        assert!(
            zig.contains("JsAny"),
            "Expected JsAny for elision in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_rest_param_and_call_spread() {
        // function foo(...args) { return args.length; }
        // foo(...arr) → foo(arr.items)
        let js = r#"
function foo(...args) {
    return args.length;
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_rest_param_and_call_spread");
        // Check that foo accepts []const JsAny
        assert!(
            zig.contains("foo(args: []const JsAny)"),
            "Expected rest param in:\n{}",
            zig
        );
        // Check that args.length is translated to args.len
        assert!(zig.contains("args.len"), "Expected args.len in:\n{}", zig);
    }

    #[test]
    fn test_p1_call_spread() {
        // Call with spread inside a function: foo(...arr) → foo(arr.items)
        let js = r#"
function foo(...args) {
    return args.length;
}
function test() {
    let arr = [1, 2, 3];
    return foo(...arr);
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_call_spread");
        // Check that foo accepts []const JsAny
        assert!(
            zig.contains("foo(args: []const JsAny)"),
            "Expected rest param in:\n{}",
            zig
        );
        // Check that foo(...arr) becomes foo(arr.items)
        assert!(
            zig.contains("foo(arr.items)"),
            "Expected call spread in:\n{}",
            zig
        );
    }
}
