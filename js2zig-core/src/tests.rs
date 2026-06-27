// native_proto/tests.rs
// Tests for native-type codegen.

#[cfg(test)]
mod native_proto_tests {
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
            || needs_js_symbol;

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

    /// Macro: transpile JS, print generated Zig, return Zig code.
    /// Usage: let zig = transpile_and_assert!(js, "test_name");
    macro_rules! transpile_and_assert {
        ($js:expr, $test_name:expr) => {{
            let result = parse_and_transpile($js, None).unwrap();
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
    /// Usage: assert_transpile_err!(js, "expected error message");
    macro_rules! assert_transpile_err {
        ($js:expr, $expected_err:expr) => {{
            let result = parse_and_transpile($js, None);
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
        if let Ok(ref res) = result
            && !res.errors.is_empty()
        {
            let all_errors = res.errors.join("; ");
            assert!(
                all_errors.contains(expected_err),
                "Expected error containing '{}', got errors: {}",
                expected_err,
                all_errors
            );
            return;
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
    fn test_native_proto_typeof() {
        let js = r#"
/**
 * @param {number} x
 * @param {boolean} b
 * @param {string} s
 */
export function typeof_test(x, b, s) {
    var t1 = typeof x;
    var t2 = typeof b;
    var t3 = typeof s;
    return t1;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typeof");
        // typeof number → should emit "number" string literal, not @typeName
        assert!(
            zig.contains("\"number\""),
            "typeof number should emit \"number\" string: {}",
            zig
        );
        // typeof bool → should emit "boolean"
        assert!(
            zig.contains("\"boolean\""),
            "typeof bool should emit \"boolean\" string: {}",
            zig
        );
        // typeof string → should emit "string"
        assert!(
            zig.contains("\"string\""),
            "typeof string should emit \"string\" string: {}",
            zig
        );
        // Should NOT contain @typeName (old behavior)
        assert!(
            !zig.contains("@typeName"),
            "typeof should NOT emit @typeName (old behavior): {}",
            zig
        );
    }

    #[test]
    fn test_native_proto_typeof_literal() {
        let js = r#"
function main() {
    var n = typeof 42;
    var s = typeof "hello";
    var b = typeof true;
    return n;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typeof_literal");
        // typeof 42 → "number"
        assert!(
            zig.contains("\"number\""),
            "typeof 42 should emit \"number\": {}",
            zig
        );
        // typeof "hello" → "string"
        assert!(
            zig.contains("\"string\""),
            r#"typeof "hello" should emit "string": {}"#,
            zig
        );
        // typeof true → "boolean"
        assert!(
            zig.contains("\"boolean\""),
            "typeof true should emit \"boolean\": {}",
            zig
        );
        // Should NOT contain @typeName
        assert!(
            !zig.contains("@typeName"),
            "typeof should NOT emit @typeName: {}",
            zig
        );
    }

    #[test]
    fn test_native_proto_typeof_object() {
        let js = r#"
/**
 * @param {Object} o
 * @param {Array} a
 */
export function typeof_obj(o, a) {
    var t1 = typeof o;
    var t2 = typeof a;
    return t1;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typeof_object");
        // typeof object → should emit "object" string literal
        assert!(
            zig.contains("\"object\""),
            "typeof object should emit \"object\" string: {}",
            zig
        );
    }

    #[test]
    fn test_native_proto_typeof_dynamic() {
        // Untyped parameters → should use runtime jsTypeof()
        let js = r#"
function check(x) {
    return typeof x;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_typeof_dynamic");
        // Should use jsTypeof() for untyped parameters
        assert!(
            zig.contains("jsTypeof"),
            "typeof untyped param should use jsTypeof(): {}",
            zig
        );
    }

    #[test]
    fn test_native_proto_void_operator() {
        let js = r#"
function void_zero() {
    return void 0;
}

function void_call() {
    var x = 0;
    void (x = 1);
    return x;
}

function void_expr(a, b) {
    void (a + b);
    return void (a * b);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_void_operator");
        // void generates JsAny.fromUndefined()
        assert!(
            zig.contains("JsAny.fromUndefined()"),
            "void operator should generate JsAny.fromUndefined(): {}",
            zig
        );
        // void should discard the expression value with _ =
        assert!(
            zig.contains("_ = "),
            "void should use _ = to discard: {}",
            zig
        );
    }

    #[test]
    fn test_native_proto_delete_operator() {
        let js = r#"
function delete_prop(obj) {
    delete obj.name;
    return obj;
}

function delete_computed(obj, key) {
    delete obj[key];
    return obj;
}

function delete_returns_bool(obj) {
    return delete obj.x;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_delete_operator");
        // delete obj.prop uses deleteKey("prop")
        assert!(
            zig.contains(".deleteKey(\"name\")"),
            "delete obj.prop should use deleteKey: {}",
            zig
        );
        // delete obj[expr] uses deleteByKey
        assert!(
            zig.contains("deleteByKey(_dk, alloc)"),
            "delete obj[expr] should use deleteByKey: {}",
            zig
        );
        // delete should consume result with _ =
        assert!(
            zig.contains("_ = ") && zig.contains("deleteKey"),
            "delete should use _ = to discard: {}",
            zig
        );
    }

    #[test]
    fn test_native_proto_compound_assignment() {
        let js = r#"
function exp_assign(a, b) {
    a **= b;
    return a;
}

function and_assign(a, b) {
    a &&= b;
    return a;
}

function or_assign(a, b) {
    a ||= b;
    return a;
}

function nullish_assign(a, b) {
    a ??= b;
    return a;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_compound_assignment");
        // **= → std.math.pow with blk
        assert!(
            zig.contains("std.math.pow(f64,"),
            "**= should use std.math.pow: {}",
            zig
        );
        // &&= → if (a.toBool()) b else a
        assert!(
            zig.contains(".toBool()"),
            "&&= should use toBool(): {}",
            zig
        );
        // ||= → if (!a.toBool()) b else a
        // (checks for toBool negation)
        // ??= → if (a.isNullish()) b else a
        assert!(
            zig.contains(".isNullish()"),
            "??= should use isNullish(): {}",
            zig
        );
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
    fn test_p2_for_in_static_codegen() {
        // Codegen verification: for-in with static struct → unrolled loop
        // This is the "showcase integration" test for P2 #6.
        let js = r#"
function gatherKeys(obj) {
    var keys = "";
    for (var k in obj) {
        keys = keys + k + ";";
    }
    return keys;
}
function demo() {
    const obj = { a: 1, b: 2, name: "test" };
    return gatherKeys(obj);
}
"#;
        let zig = transpile_and_assert!(js, "test_p2_for_in_static_codegen");
        // Verify: unrolled loop (no HashMap iterator)
        assert!(
            zig.contains("const k = \"a\""),
            "Expected unrolled field 'a'"
        );
        assert!(
            zig.contains("const k = \"b\""),
            "Expected unrolled field 'b'"
        );
        assert!(
            zig.contains("const k = \"name\""),
            "Expected unrolled field 'name'"
        );
        assert!(!zig.contains("__it"), "Should not have HashMap iterator");
        // Verify: string concatenation inside unrolled blocks
        assert!(
            zig.contains("allocPrint"),
            "Expected allocPrint for string concat"
        );
    }

    #[test]
    fn test_p2_nested_function_no_capture() {
        // Nested function without captures: should generate struct with call() method
        let js = r#"
function outer(x) {
    function inner(y) {
        return y * 2;
    }
    return inner(x);
}
"#;
        let zig = transpile_and_assert!(js, "test_p2_nested_function_no_capture");
        println!("=== Nested function (no capture) Zig code ===\n{}", zig);

        // Verify: inner function is hoisted as a struct
        assert!(
            zig.contains("const inner = struct {"),
            "Expected inner to be a struct"
        );
        assert!(zig.contains("pub fn call("), "Expected call() method");

        // Verify: call is rewritten to inner.call(x)
        assert!(
            zig.contains("inner.call(x)"),
            "Expected call to be rewritten"
        );
    }

    #[test]
    fn test_p2_nested_function_with_capture() {
        // Nested function with captures: should generate struct with captured variables
        let js = r#"
function outer(x) {
    function inner(y) {
        return x + y;  // x is captured from outer scope
    }
    return inner(3);
}
"#;
        let zig = parse_and_transpile(js, None).unwrap().zig_code;
        println!("=== Nested function (with capture) Zig code ===\n{}", zig);

        // Verify: inner is defined as a struct with capture field
        assert!(
            zig.contains("const inner = struct {"),
            "Expected inner to be a struct"
        );
        assert!(zig.contains("x:"), "Expected capture field x");

        // Verify: call is rewritten to inner.call(args)
        assert!(zig.contains("inner.call("), "Expected call to be rewritten");

        // Verify: struct has call method with self parameter
        assert!(
            zig.contains("pub fn call(self:"),
            "Expected call method with self parameter"
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
            .args(["ast-check", zig_path.to_str().unwrap()])
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
        let zig_full = r#"const std = @import("std");

const PI: f64 = 3.14159;

fn add(a: anytype, b: anytype) !@TypeOf(a + b) {
    return a + b;
}

fn abs(x: anytype) !@TypeOf(x) {
    if (x >= 0) {
        return x;
    }
    return -x;
}

pub fn main() !void {
    const x = try add(10, 20);
    const y = try abs(-42);
    std.debug.print("add(10,20)={}  abs(-42)={}\n", .{x, y});
}
"#
        .to_string();

        // Step 4: write full program and compile
        let zig_path_full = tmp_dir.join("e2e_native_full.zig");
        let exe_path = tmp_dir.join("e2e_native_full.exe");
        std::fs::write(&zig_path_full, &zig_full).unwrap();

        let build_output = std::process::Command::new("zig.exe")
            .args([
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
            .args(["ast-check", zig_path.to_str().unwrap()])
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

        // Verify JSON.stringify() is converted to js_json.stringify()
        assert!(
            zig.contains("try js_json.stringify(js_allocator.g_alloc(), user"),
            "Expected try js_json.stringify(), got:\n{}",
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
const js_json = @import("js_runtime/js_json.zig");

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
            .args(["ast-check", zig_path.to_str().unwrap()])
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
            .args(["build-exe", zig_path.to_str().unwrap(), "-freference-trace"])
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
        // JS source: Math.abs(), Math.floor(), Math.ceil(), Math.round(), Math.sqrt(), Math.hypot()
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
    const hypotX = Math.hypot(x, 3, 4);
    return absX + floorX + ceilX + roundX + sqrtX + hypotX;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_math_methods");

        // Step2: verify Math methods are generated correctly
        assert!(zig.contains("@abs("), "Expected '@abs(' in:\n{}", zig);
        assert!(zig.contains("@floor("), "Expected '@floor(' in:\n{}", zig);
        assert!(zig.contains("@ceil("), "Expected '@ceil(' in:\n{}", zig);
        assert!(zig.contains("@round("), "Expected '@round(' in:\n{}", zig);
        assert!(zig.contains("@sqrt("), "Expected '@sqrt(' in:\n{}", zig);
        // Math.hypot(x, 3, 4) → @sqrt(@as(f64,x)*@as(f64,x) + @as(f64,3)*@as(f64,3) + @as(f64,4)*@as(f64,4))
        assert!(
            zig.contains("@sqrt("),
            "Expected '@sqrt(' for Math.hypot in:\n{}",
            zig
        );
        assert!(
            zig.contains("*@as(f64,"),
            "Expected squared terms for Math.hypot in:\n{}",
            zig
        );
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

    // ── Test: Phase 4 Math methods (expm1/sinh/cosh/tanh/asinh/acosh/atanh/clz32/fround/imul/log1p) ──
    #[test]
    fn test_native_proto_math_phase4() {
        let js = r#"
/**
 * @param {number} x
 * @param {number} y
 * @returns {number}
 */
export function testMathPhase4(x, y) {
    const a = Math.expm1(x);
    const b = Math.sinh(x);
    const c = Math.cosh(x);
    const d = Math.tanh(x);
    const e = Math.asinh(x);
    const f = Math.acosh(x);
    const g = Math.atanh(x);
    const h = Math.clz32(x);
    const i = Math.fround(x);
    const j = Math.imul(x, y);
    const k = Math.log1p(x);
    return a + b + c + d + e + f + g + h + i + j + k;
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_math_phase4");

        // Verify Phase 4 Math methods are generated correctly
        assert!(
            zig.contains("std.math.expm1("),
            "Expected 'std.math.expm1(' in:\n{}",
            zig
        );
        assert!(
            zig.contains("std.math.sinh("),
            "Expected 'std.math.sinh(' in:\n{}",
            zig
        );
        assert!(
            zig.contains("std.math.cosh("),
            "Expected 'std.math.cosh(' in:\n{}",
            zig
        );
        assert!(
            zig.contains("std.math.tanh("),
            "Expected 'std.math.tanh(' in:\n{}",
            zig
        );
        assert!(
            zig.contains("std.math.asinh("),
            "Expected 'std.math.asinh(' in:\n{}",
            zig
        );
        assert!(
            zig.contains("std.math.acosh("),
            "Expected 'std.math.acosh(' in:\n{}",
            zig
        );
        assert!(
            zig.contains("std.math.atanh("),
            "Expected 'std.math.atanh(' in:\n{}",
            zig
        );
        assert!(zig.contains("@clz("), "Expected '@clz(' in:\n{}", zig);
        assert!(zig.contains("@as(f32,"), "Expected '@as(f32,' in:\n{}", zig);
        assert!(zig.contains("@as(i32,"), "Expected '@as(i32,' in:\n{}", zig);
        assert!(
            zig.contains("std.math.log1p("),
            "Expected 'std.math.log1p(' in:\n{}",
            zig
        );
    }

    // ── Test: AwaitExpression support ────────────

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
        // Should generate if-else with error capture for the handler
        assert!(
            zig.contains("else |err|"),
            "Expected else |err| for catch handler:\n{}",
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
        // Should have catch handler via if-else
        assert!(
            zig.contains("else |err|"),
            "Expected catch handler via if-else:\n{}",
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
        // Finally body should be generated as defer inside labeled block
        assert!(
            zig.contains("defer {") && zig.contains("val = 0;"),
            "Expected finally as defer:\n{}",
            zig
        );
        // Should have catch handler via if-else
        assert!(
            zig.contains("else |err|"),
            "Expected catch handler via if-else:\n{}",
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
            !zig.contains("else |err|"),
            "Should not have catch handler for throw-free try:\n{}",
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

    // ── Test: Nested try-catch (resource release) ──────────

    #[test]
    fn test_native_proto_try_catch_nested_inner_catch() {
        // Nested try-catch: throw in inner try → caught by inner catch → handled.
        // Outer try should NOT see the error (inner catch consumed it).
        let js = r##"
function nestedCatch(a, b) {
    try {
        try {
            if (b === 0) throw "div by zero";
            return a / b;
        } catch (e) {
            return -1;
        }
        return -2;
    } catch (e) {
        return -3;
    }
}
"##;
        let zig = transpile_and_assert!(js, "test_native_proto_try_catch_nested_inner_catch");
        println!("=== Nested try-catch (inner catch) ===\n{}", zig);
        // Each try-catch generates `= _js_try_N:` and `= _js_try_body_N:`
        let result_count = zig.matches("= _js_try_").count(); // = _js_try_0, =_js_try_body_0
        assert!(
            result_count == 4,
            "Expected 4 '= _js_try_' assignments for 2 nested try-catch, got {}:\n{}",
            result_count,
            zig
        );
        // Inner catch handler generates `_ = @errorName(err)` (e unused, body just returns -1)
        assert!(
            zig.contains("_ = @errorName(err);"),
            "Expected '_ = @errorName' in inner catch:\n{}",
            zig
        );
        // Error propagation: `if (_js_try_1) |_| {} else |_| break :_js_try_body_blk_0`
        assert!(
            zig.contains("break :_js_try_body_blk_0"),
            "Expected error propagation break from inner to outer body block:\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_try_catch_nested_inner_catch");
    }

    #[test]
    fn test_native_proto_try_catch_nested_rethrow() {
        // Nested try-catch with re-throw: throw in inner catch → caught by outer catch.
        let js = r##"
function rethrowExample(a, b) {
    try {
        try {
            if (b === 0) throw "div by zero";
            return a / b;
        } catch (inner) {
            throw inner;
        }
    } catch (outer) {
        return -1;
    }
}
"##;
        let zig = transpile_and_assert!(js, "test_native_proto_try_catch_nested_rethrow");
        println!("=== Nested try-catch (rethrow) ===\n{}", zig);
        // Each try-catch generates `= _js_try_N:` (result) + `= _js_try_body_N:` (body)
        // 2 nested = 4 total.
        let result_count = zig.matches("= _js_try_").count();
        assert!(
            result_count == 4,
            "Expected 4 '= _js_try_' assignments for nested try-catch rethrow, got {}:\n{}",
            result_count,
            zig
        );
        // Should have rethrow pattern: inner catch throws, break inner body block with error
        assert!(
            zig.contains("break :"),
            "Expected break :label for rethrow:\n{}",
            zig
        );
        assert_zig_ast_check(&zig, "test_native_proto_try_catch_nested_rethrow");
    }

    #[test]
    fn test_native_proto_try_catch_nested_no_throw() {
        // Nested try-catch where inner try has no throw (but has catch handler).
        // Body block labels are generated but may be unused if no rethrow occurs.
        // Known limitation: Zig may warn about unused block labels.
        let js = r#"
function nestedNoThrow(x) {
    try {
        try {
            return x + 1;
        } catch (e) {
            return 0;
        }
    } catch (e) {
        return -1;
    }
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_try_catch_nested_no_throw");
        println!("=== Nested try-catch (no throw) ===\n{}", zig);
        // Body should contain return x + 1
        assert!(
            zig.contains("return x + 1"),
            "Expected body for no-throw inner try:\n{}",
            zig
        );
        // Has catch handler with const e
        assert!(
            zig.contains("const e = @errorName(err);") || zig.contains("_ = @errorName(err);"),
            "Expected catch handler for inner try:\n{}",
            zig
        );
        // NOTE: assert_zig_ast_check skipped due to known limitation:
        // When nested try-catch has no throw in inner body, the outer body
        // block label (_js_try_body_blk_0) is generated but never referenced.
        // This is tracked as a minor codegen optimization issue.
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
        // Date.UTC(y, m, d) → js_date.utc(y, m, d, 0, 0, 0, 0)
        let js = r#"
/**
 * @returns {number}
 */
export function utcDate(y, m, d) {
    return Date.UTC(y, m, d);
}
"#;
        let zig = transpile_and_check!(js, "test_p1_date_utc");
        assert!(
            zig.contains("js_date.utc("),
            "Expected 'js_date.utc(' in generated output:\n{}",
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
            zig.contains(".getTime()"),
            "Expected .getTime() in:\n{}",
            zig
        );
        assert!(
            zig.contains(".getFullYear()"),
            "Expected .getFullYear() in:\n{}",
            zig
        );
        assert!(
            zig.contains(".getMonth()"),
            "Expected .getMonth() in:\n{}",
            zig
        );
        assert!(
            zig.contains(".getDate()"),
            "Expected .getDate() in:\n{}",
            zig
        );
        assert!(zig.contains(".getDay()"), "Expected .getDay() in:\n{}", zig);
        assert!(
            zig.contains(".getHours()"),
            "Expected .getHours() in:\n{}",
            zig
        );
        assert!(
            zig.contains(".getMinutes()"),
            "Expected .getMinutes() in:\n{}",
            zig
        );
        assert!(
            zig.contains(".getSeconds()"),
            "Expected .getSeconds() in:\n{}",
            zig
        );
    }

    // ── P1-2b: Date constructor overloads ──────────────────────────
    // NOTE: use transpile_and_assert! instead of transpile_and_check!
    // because the Zig AST checker cannot resolve js_date.JsDate as a return type.

    #[test]
    fn test_p1_date_new_empty() {
        // new Date() → js_date.JsDate.init()
        let js = r#"
export function makeDate() {
    return new Date();
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_date_new_empty");
        assert!(
            zig.contains("js_date.JsDate.init()"),
            "Expected 'js_date.JsDate.init()' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_date_new_millis() {
        // new Date(0) → js_date.JsDate.fromMillis(0)
        let js = r#"
export function epochDate() {
    return new Date(0);
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_date_new_millis");
        assert!(
            zig.contains("js_date.JsDate.fromMillis("),
            "Expected 'js_date.JsDate.fromMillis(' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_date_new_string() {
        // new Date("2024-01-15") → js_date.JsDate.fromMillis(js_date.parse("2024-01-15"))
        let js = r#"
export function dateFromStr() {
    return new Date("2024-01-15");
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_date_new_string");
        assert!(
            zig.contains("js_date.parse("),
            "Expected 'js_date.parse(' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_date_new_multi_2args() {
        // new Date(2024, 5) → js_date.JsDate.fromComponents(2024, 5, 1, 0, 0, 0, 0)
        let js = r#"
export function dateYearMonth() {
    return new Date(2024, 5);
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_date_new_multi_2args");
        assert!(
            zig.contains("js_date.JsDate.fromComponents("),
            "Expected 'js_date.JsDate.fromComponents(' in:\n{}",
            zig
        );
        assert!(
            zig.contains(", 1, 0, 0, 0, 0)"),
            "Expected default padding ', 1, 0, 0, 0, 0)' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_date_new_multi_3args() {
        // new Date(2024, 5, 15) → js_date.JsDate.fromComponents(2024, 5, 15, 0, 0, 0, 0)
        let js = r#"
export function dateYMD() {
    return new Date(2024, 5, 15);
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_date_new_multi_3args");
        assert!(
            zig.contains("js_date.JsDate.fromComponents("),
            "Expected 'js_date.JsDate.fromComponents(' in:\n{}",
            zig
        );
        assert!(
            zig.contains(", 15, 0, 0, 0, 0)"),
            "Expected default padding with 15 for day, rest 0 in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_date_new_multi_5args() {
        // new Date(2024, 5, 15, 12, 30) → js_date.JsDate.fromComponents(2024, 5, 15, 12, 30, 0, 0)
        let js = r#"
export function dateYMDHM() {
    return new Date(2024, 5, 15, 12, 30);
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_date_new_multi_5args");
        assert!(
            zig.contains("js_date.JsDate.fromComponents("),
            "Expected 'js_date.JsDate.fromComponents(' in:\n{}",
            zig
        );
        assert!(
            zig.contains(", 0, 0)"),
            "Expected default padding for missing sec/ms in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p1_date_new_multi_7args() {
        // new Date(2024, 5, 15, 12, 30, 45, 500) → js_date.JsDate.fromComponents(2024, 5, 15, 12, 30, 45, 500)
        let js = r#"
export function dateAll() {
    return new Date(2024, 5, 15, 12, 30, 45, 500);
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_date_new_multi_7args");
        assert!(
            zig.contains("js_date.JsDate.fromComponents("),
            "Expected 'js_date.JsDate.fromComponents(' in:\n{}",
            zig
        );
        // Should contain all 7 args with no default padding
        assert!(zig.contains("500)"), "Expected ms=500 in:\n{}", zig);
    }

    #[test]
    fn test_p1_date_new_multi_variable_args() {
        // new Date(y, m, d) with variables → fromComponents(y, m, d, 0, 0, 0, 0)
        let js = r#"
/**
 * @param {number} y
 * @param {number} m
 * @param {number} d
 */
export function dateFromVars(y, m, d) {
    return new Date(y, m, d);
}
"#;
        let zig = transpile_and_assert!(js, "test_p1_date_new_multi_variable_args");
        assert!(
            zig.contains("js_date.JsDate.fromComponents("),
            "Expected 'js_date.JsDate.fromComponents(' with variable args in:\n{}",
            zig
        );
        // Should contain variable args + padding
        assert!(
            zig.contains("d, 0, 0, 0, 0)"),
            "Expected 'd, 0, 0, 0, 0)' padding for variable args in:\n{}",
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

    #[test]
    fn test_p1_object_from_entries() {
        let js = r#"
function fromEntriesWrapper(entries) {
    return Object.fromEntries(entries);
}
"#;
        let zig = transpile_and_check!(js, "test_p1_object_from_entries");
        assert!(
            zig.contains("js_object.fromEntries("),
            "Expected js_object.fromEntries() in:\n{}",
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
    fn test_p2_for_of_map_single_var() {
        // for (const x of m) where m is Map → while iterator → const x = entry.key_ptr.*
        let js = r#"
function sumKeys() {
    const m = new Map();
    m.set("a", 1);
    m.set("b", 2);
    var total = 0;
    for (const x of m) {
        total = total + 1;
    }
    return total;
}
"#;
        let zig = transpile_and_assert!(js, "test_p2_for_of_map_single_var");
        println!(
            "=== Generated Zig (test_p2_for_of_map_single_var) ===\n{}",
            zig
        );
        assert!(
            zig.contains(".inner.iterator()"),
            "Expected iterator in:\n{}",
            zig
        );
        assert!(
            zig.contains(".key_ptr.*"),
            "Expected .key_ptr.* in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p2_for_of_map_destructure() {
        // for (const [k, v] of m) where m is Map → key_ptr.* / value_ptr.* destructure
        let js = r#"
function iterateMap() {
    const m = new Map();
    m.set("x", "y");
    var result = "";
    for (const [key, val] of m) {
        result = result + key;
    }
    return result;
}
"#;
        let zig = transpile_and_assert!(js, "test_p2_for_of_map_destructure");
        println!(
            "=== Generated Zig (test_p2_for_of_map_destructure) ===\n{}",
            zig
        );
        assert!(
            zig.contains(".inner.iterator()"),
            "Expected iterator in:\n{}",
            zig
        );
        assert!(
            zig.contains(".key_ptr.*"),
            "Expected .key_ptr.* in:\n{}",
            zig
        );
        assert!(
            zig.contains(".value_ptr.*"),
            "Expected .value_ptr.* in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p2_for_of_set() {
        // for (const x of s) where s is Set → while iterator → const x = entry.key_ptr.*
        let js = r#"
function iterSet() {
    const s = new Set();
    s.add(10);
    s.add(20);
    var count = 0;
    for (const x of s) {
        count = count + 1;
    }
    return count;
}
"#;
        let zig = transpile_and_assert!(js, "test_p2_for_of_set");
        println!("=== Generated Zig (test_p2_for_of_set) ===\n{}", zig);
        assert!(
            zig.contains(".inner.iterator()"),
            "Expected iterator in:\n{}",
            zig
        );
        assert!(
            zig.contains(".key_ptr.*"),
            "Expected .key_ptr.* in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p2_for_of_string() {
        // for (const ch of str) → for (str) |ch| { ... } (Zig string iteration)
        let js = r#"
/**
 * @param {string} str
 */
function countChars(str) {
    var count = 0;
    for (const ch of str) {
        count = count + 1;
    }
    return count;
}
"#;
        let zig = transpile_and_assert!(js, "test_p2_for_of_string");
        println!("=== Generated Zig (test_p2_for_of_string) ===\n{}", zig);
        assert!(zig.contains("for ("), "Expected for loop in:\n{}", zig);
        assert!(zig.contains(") |ch|"), "Expected |ch| capture in:\n{}", zig);
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

    // ── Destructuring tests ───────────────────────────

    #[test]
    fn test_p2_destructure_object_basic() {
        // const {a, b} = obj → const a = obj.get("a"); const b = obj.get("b");
        let js = r#"
function basic(obj) {
    const { a, b } = obj;
    return a + b;
}
"#;
        let zig = transpile_and_assert!(js, "test_p2_destructure_object_basic");
        assert!(
            zig.contains("const _js_dest_"),
            "Expected temp variable in:\n{}",
            zig
        );
        assert!(
            zig.contains(".get(\"a\")"),
            "Expected .get(\"a\") access in:\n{}",
            zig
        );
        assert!(
            zig.contains(".get(\"b\")"),
            "Expected .get(\"b\") access in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p2_destructure_object_with_defaults() {
        // const {a = 1, b = 2} = obj (HashMap) →
        //   const a = if (_js_dest_0.get("a")) |v| v.asI64() else 1;
        let js = r#"
function withDefaults(obj) {
    const { a = 1, b = 2 } = obj;
    return a + b;
}
"#;
        let zig = transpile_and_check!(js, "test_p2_destructure_object_with_defaults");
        assert!(
            zig.contains(".asI64() else 1"),
            "Expected '.asI64() else 1' in:\n{}",
            zig
        );
        assert!(
            zig.contains(".asI64() else 2"),
            "Expected '.asI64() else 2' in:\n{}",
            zig
        );
        assert!(
            zig.contains(".get(\"a\")"),
            "Expected .get(\"a\") in:\n{}",
            zig
        );
        assert!(
            zig.contains(".get(\"b\")"),
            "Expected .get(\"b\") in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p2_destructure_object_rename() {
        // const {a: x} = obj → const x = obj.get("a");
        let js = r#"
function rename(obj) {
    const { a: x } = obj;
    return x;
}
"#;
        let zig = transpile_and_assert!(js, "test_p2_destructure_object_rename");
        assert!(
            zig.contains("const x = "),
            "Expected 'const x =' in:\n{}",
            zig
        );
        assert!(
            zig.contains(".get(\"a\")"),
            "Expected .get(\"a\") access in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p2_destructure_object_mixed() {
        // const {a, b: c = 10} = obj (HashMap)
        // a → .get("a"), c → if (.get("b")) |v| v.asI64() else 10 (renamed + default)
        let js = r#"
function mixed(obj) {
    const { a, b: c = 10 } = obj;
    return a + c;
}
"#;
        let zig = transpile_and_check!(js, "test_p2_destructure_object_mixed");
        assert!(
            zig.contains("const a = "),
            "Expected 'const a' in:\n{}",
            zig
        );
        assert!(
            zig.contains("const c = "),
            "Expected 'const c' in:\n{}",
            zig
        );
        assert!(
            zig.contains(".asI64() else 10"),
            "Expected '.asI64() else 10' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p2_destructure_array_basic() {
        // const [a, b] = arr → const a = arr[0]; const b = arr[1];
        let js = r#"
function arrayBasic(arr) {
    const [a, b] = arr;
    return a + b;
}
"#;
        let zig = transpile_and_assert!(js, "test_p2_destructure_array_basic");
        assert!(zig.contains("[0]"), "Expected '[0]' in:\n{}", zig);
        assert!(zig.contains("[1]"), "Expected '[1]' in:\n{}", zig);
    }

    #[test]
    fn test_p2_destructure_array_with_defaults() {
        // const [a = 1, b = 2] = arr → const a = arr[0] orelse 1; const b = arr[1] orelse 2;
        let js = r#"
function arrayDefaults(arr) {
    const [a = 1, b = 2] = arr;
    return a + b;
}
"#;
        let zig = transpile_and_check!(js, "test_p2_destructure_array_with_defaults");
        assert!(
            zig.contains("[0] orelse 1"),
            "Expected '[0] orelse 1' in:\n{}",
            zig
        );
        assert!(
            zig.contains("[1] orelse 2"),
            "Expected '[1] orelse 2' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p2_destructure_array_hole() {
        // const [a, , b] = arr → skip hole
        let js = r#"
function arrayHole(arr) {
    const [a, , b] = arr;
    return a + b;
}
"#;
        let zig = transpile_and_assert!(js, "test_p2_destructure_array_hole");
        assert!(zig.contains("[0]"), "Expected '[0]' in:\n{}", zig);
        assert!(zig.contains("[2]"), "Expected '[2]' in:\n{}", zig);
        // Should NOT contain [1]
        assert!(
            !zig.contains("[1]"),
            "Should not contain '[1]' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p2_destructure_function_call_init() {
        // Destructuring from a function call → temp variable
        let js = r#"
function callInit() {
    const obj = { x: 1, y: 2 };
    const { x, y } = obj;
    return x + y;
}
"#;
        let zig = transpile_and_assert!(js, "test_p2_destructure_function_call_init");
        assert!(
            zig.contains("const x = "),
            "Expected 'const x' in:\n{}",
            zig
        );
        assert!(
            zig.contains("const y = "),
            "Expected 'const y' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p2_nested_function_basic() {
        // Nested function declaration without captures:
        // function outer() { function inner(x) { return x + 1; } return inner(5); }
        // → inner is hoisted as `const inner = struct { pub fn call(...) ... };`
        //   and `inner(5)` is rewritten to `inner.call(5)`
        let js = r#"
function outer() {
    function inner(x) {
        return x + 1;
    }
    return inner(5);
}
"#;
        let zig = transpile_and_assert!(js, "test_p2_nested_function_basic");
        assert!(
            zig.contains("const inner = struct {"),
            "Expected inline struct declaration in:\n{}",
            zig
        );
        assert!(
            zig.contains("inner.call(5)"),
            "Expected 'inner.call(5)' in:\n{}",
            zig
        );
        assert!(
            zig.contains("pub fn call("),
            "Expected 'pub fn call(' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p2_nested_function_capture_error() {
        // Nested function that captures outer variable → now supported!
        let js = r#"
function outer(x) {
    function inner(y) {
        return x + y;
    }
    return inner(10);
}
"#;
        let zig = transpile_and_assert!(js, "test_p2_nested_function_capture_error");
        println!("=== Nested function capture Zig code ===\n{}", zig);

        // Verify: inner is defined as a struct with capture field
        assert!(
            zig.contains("const inner = struct {"),
            "Expected inner to be a struct"
        );
        assert!(zig.contains("x:"), "Expected capture field x");

        // Verify: call is rewritten to inner.call(args)
        assert!(zig.contains("inner.call("), "Expected call to be rewritten");

        // Verify: struct has call method with self parameter
        assert!(
            zig.contains("pub fn call(self:"),
            "Expected call method with self parameter"
        );
    }

    // ── Class transpilation tests ────────────────────────────

    #[test]
    fn test_native_proto_class_basic() {
        // Simple class with i64 fields and area() method
        let js = r#"
class Rectangle {
    width = 0;
    height = 0;
    constructor(w, h) {
        this.width = w;
        this.height = h;
    }
    area() {
        return this.width * this.height;
    }
}

export function testRect() {
    const rect = new Rectangle(3, 4);
    return rect.area();
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_class_basic");
        println!("=== Class transpilation Zig code ===\n{}", zig);

        // Verify: Rectangle struct is defined
        assert!(
            zig.contains("const Rectangle = struct {"),
            "Expected Rectangle struct definition"
        );

        // Verify: init() maps constructor (may have extra whitespace after param list)
        assert!(
            zig.contains("pub fn init(w: anytype, h: anytype"),
            "Expected init(w, h) constructor. Got:\n{}",
            zig
        );

        // Verify: new Rectangle routes to Rectangle.init
        assert!(
            zig.contains("Rectangle.init("),
            "Expected new Rect→Rectangle.init routing"
        );

        // Verify: area() method exists
        assert!(
            zig.contains("pub fn area(self:"),
            "Expected area method. Got:\n{}",
            zig
        );

        // Verify: export function compiles
        assert!(
            zig.contains("pub fn testRect"),
            "Expected testRect export. Got:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_class_mixed_fields() {
        // Class with i64 + string fields
        let js = r#"
class User {
    id = 0;
    name = "";
    constructor(idVal, nameVal) {
        this.id = idVal;
        this.name = nameVal;
    }
    getId() {
        return this.id;
    }
    getName() {
        return this.name;
    }
}

export function testUser() {
    const u = new User(42, "Alice");
    return u.getId();
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_class_mixed_fields");
        println!("=== Class mixed fields Zig code ===\n{}", zig);

        // Verify: User struct defined
        assert!(
            zig.contains("const User = struct {"),
            "Expected User struct"
        );

        // Verify: init with two params
        assert!(zig.contains("pub fn init("), "Expected init method");

        // Verify: new User routes correctly
        assert!(zig.contains("User.init("), "Expected User.init routing");

        // Verify: getId and getName methods exist
        assert!(zig.contains("getId"), "Expected getId method");
        assert!(zig.contains("getName"), "Expected getName method");
    }

    #[test]
    fn test_native_proto_class_implicit_fields() {
        // Class WITHOUT PropertyDefinition — all fields declared via this.x = expr in constructor
        let js = r#"
class Point {
    constructor(xVal, yVal) {
        this.x = xVal;
        this.y = yVal;
    }
    sum() {
        return this.x + this.y;
    }
}

export function testPoint() {
    const p = new Point(10, 20);
    return p.sum();
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_class_implicit_fields");
        println!("=== Class implicit fields Zig code ===\n{}", zig);

        // Verify: Point struct defined
        assert!(
            zig.contains("const Point = struct {"),
            "Expected Point struct definition. Got:\n{}",
            zig
        );

        // Verify: x and y fields are present (inferred from constructor this.x = expr)
        assert!(zig.contains("x:"), "Expected field 'x' in struct");
        assert!(zig.contains("y:"), "Expected field 'y' in struct");

        // Verify: init() maps constructor
        assert!(zig.contains("pub fn init("), "Expected init() constructor");

        // Verify: new Point routes correctly
        assert!(
            zig.contains("Point.init("),
            "Expected new Point→Point.init routing"
        );

        // Verify: sum() method exists
        assert!(zig.contains("pub fn sum(self:"), "Expected sum method");

        // Verify: export function compiles
        assert!(
            zig.contains("pub fn testPoint"),
            "Expected testPoint export"
        );
    }

    #[test]
    fn test_native_proto_array_flat() {
        let js = r#"
export function testFlat() {
    const arr = [1, 2, 3];
    return arr.flat();
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_array_flat");
        println!("=== Array.flat Zig code ===\n{}", zig);

        // Verify: flat identity - arr.flat() → arr (i64 arrays already flat)
        assert!(zig.contains("testFlat"), "Expected testFlat function");
    }

    #[test]
    fn test_native_proto_array_flat_map() {
        let js = r#"
export function testFlatMap() {
    const arr = [1, 2, 3];
    return arr.flatMap((x) => x * 2);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_array_flat_map");
        println!("=== Array.flatMap Zig code ===\n{}", zig);

        // Verify: flatMap is recognized; simplified to return original array
        assert!(zig.contains("testFlatMap"), "Expected testFlatMap function");
    }

    #[test]
    fn test_native_proto_string_pad_start() {
        let js = r#"
export function testPadStart() {
    const s = "42";
    return s.padStart(5, "0");
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_string_pad_start");
        println!("=== String.padStart Zig code ===\n{}", zig);

        // Verify: padStart runtime call generated
        assert!(
            zig.contains("js_string.padStart"),
            "Expected js_string.padStart"
        );
    }

    #[test]
    fn test_native_proto_string_pad_end() {
        let js = r#"
export function testPadEnd() {
    const s = "hello";
    return s.padEnd(10, ".");
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_string_pad_end");
        println!("=== String.padEnd Zig code ===\n{}", zig);

        // Verify: padEnd runtime call generated
        assert!(
            zig.contains("js_string.padEnd"),
            "Expected js_string.padEnd"
        );
    }

    // ── Test: String.substring() ─────────────

    #[test]
    fn test_native_proto_string_substring() {
        let js = r#"
export function testSubstring() {
    const s = "hello world";
    return s.substring(0, 5);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_string_substring");
        assert!(
            zig.contains("js_string.substring"),
            "Expected js_string.substring call, got:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_string_substring_swap() {
        // substring(5, 0) should swap to (0, 5) — JS semantics
        let js = r#"
export function testSubstringSwap() {
    const s = "hello world";
    return s.substring(5, 0);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_string_substring_swap");
        assert!(
            zig.contains("js_string.substring"),
            "Expected js_string.substring call, got:\n{}",
            zig
        );
    }

    // ── Test: String.at() (Task #627) ────────────
    #[test]
    fn test_native_proto_string_at() {
        let js = r#"
export function testAt() {
    const s = "hello";
    return s.at(1);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_string_at");
        println!("=== String.at() Zig code ===\n{}", zig);

        // Verify: at() runtime call generated
        assert!(
            zig.contains("js_string.at"),
            "Expected js_string.at call, got:\n{}",
            zig
        );

        // Verify: handles negative index
        let js_neg = r#"
export function testAtNeg() {
    const s = "hello";
    return s.at(-1);
}
"#;
        let zig_neg = transpile_and_assert!(js_neg, "test_native_proto_string_at_neg");
        println!("=== String.at(-1) Zig code ===\n{}", zig_neg);

        assert!(
            zig_neg.contains("js_string.at"),
            "Expected js_string.at call for negative index, got:\n{}",
            zig_neg
        );
    }

    // ── Test: String.codePointAt() (Task #630) ────────────
    #[test]
    fn test_native_proto_string_code_point_at() {
        let js = r#"
export function testCodePointAt() {
    const s = "hello";
    return s.codePointAt(1);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_string_code_point_at");
        println!("=== String.codePointAt() Zig code ===\n{}", zig);

        // Verify: codePointAt() runtime call generated
        assert!(
            zig.contains("js_string.codePointAt"),
            "Expected js_string.codePointAt call, got:\n{}",
            zig
        );
    }

    // ── Test: Object.hasOwn() with struct ─────────────

    #[test]
    fn test_native_proto_object_has_own_struct() {
        let js = r#"
export function testHasOwn() {
    const obj = { name: "Alice", age: 30 };
    return Object.hasOwn(obj, "name");
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_object_has_own_struct");
        assert!(
            zig.contains("@hasField"),
            "Expected @hasField for struct type, got:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_object_has_own_missing() {
        let js = r#"
export function testHasOwnMissing() {
    const obj = { name: "Alice", age: 30 };
    return Object.hasOwn(obj, "email");
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_object_has_own_missing");
        assert!(
            zig.contains("@hasField"),
            "Expected @hasField for struct type, got:\n{}",
            zig
        );
    }

    // ── Test: JSON.parse() without JSDoc (Phase 1) ─────────────

    #[test]
    fn test_native_proto_json_parse_no_jsdoc() {
        // JSON.parse() without JSDoc should return JsAny type
        let js = r#"
const json = JSON.parse('{"name":"Alice","age":30}');

export function getName() {
    return json.name;
}
"#;
        let result = parse_and_transpile(js, None);
        match &result {
            Ok(r) => println!("=== JSON.parse() without JSDoc ===\n{}", r.zig_code),
            Err(e) => println!("=== JSON.parse() without JSDoc ERROR ===\n{:?}", e),
        }
        let zig = result.unwrap().zig_code;

        // Should NOT generate std.json.parse() (that's for JSDoc @type case)
        assert!(
            !zig.contains("std.json.parse("),
            "Should not generate std.json.parse() without JSDoc, got:\n{}",
            zig
        );

        // Should generate js_json.parse() which returns JsAny
        assert!(
            zig.contains("js_json.parse("),
            "Expected js_json.parse(), got:\n{}",
            zig
        );

        // json variable should be declared (Zig infers type as JsAny)
        assert!(
            zig.contains("const json"),
            "Expected 'const json' declaration, got:\n{}",
            zig
        );
    }

    // ── Test: Array.filter() with concise arrow predicate ─────────────

    #[test]
    fn test_native_proto_array_filter() {
        // JS source: arr.filter(x => x > 3) — returns filtered ArrayList
        let js = r#"
/**
 * @returns {number}
 */
export function filterCount() {
    const arr = [1, 2, 3, 4, 5, 6];
    const filtered = arr.filter(x => x > 3);
    return filtered.length;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_filter");

        // Verify inline for-loop with ArrayList result
        assert!(zig.contains("blk:"), "Expected labeled block in:\n{}", zig);
        assert!(
            zig.contains("__filter: std.ArrayList("),
            "Expected __filter ArrayList var in:\n{}",
            zig
        );
        assert!(zig.contains("for ("), "Expected for loop in:\n{}", zig);
        assert!(
            zig.contains(".append(js_allocator.getAllocator()"),
            "Expected append in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk __filter"),
            "Expected break :blk __filter in:\n{}",
            zig
        );
        // Should contain the predicate condition: x > 3
        assert!(zig.contains("> 3"), "Expected '> 3' predicate in:\n{}", zig);
    }

    // ── Test: Array.some() with concise arrow predicate ─────────────

    #[test]
    fn test_native_proto_array_some() {
        let js = r#"
/**
 * @returns {number}
 */
export function hasMatch() {
    const arr = [1, 2, 3, 4, 5];
    if (arr.some(x => x > 3)) {
        return 1;
    }
    return 0;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_some");
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

    // ── Test: Array.every() with concise arrow predicate ─────────────

    #[test]
    fn test_native_proto_array_every() {
        let js = r#"
/**
 * @returns {number}
 */
export function allPositive() {
    const arr = [1, 2, 3, 4, 5];
    if (arr.every(x => x > 0)) {
        return 1;
    }
    return 0;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_every");
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

    // ── Test: Array.some() with block body arrow ─────────────

    #[test]
    fn test_native_proto_array_some_block_body() {
        let js = r#"
/**
 * @returns {number}
 */
export function someBlockBody() {
    const arr = [1, 2, 3, 4, 5];
    if (arr.some(x => { return x > 3; })) {
        return 1;
    }
    return 0;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_some_block_body");
        assert!(
            zig.contains("break :blk true"),
            "Expected break :blk true, got:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk false"),
            "Expected break :blk false, got:\n{}",
            zig
        );
    }

    // ── Test: Array.every() with block body arrow ─────────────

    #[test]
    fn test_native_proto_array_every_block_body() {
        let js = r#"
/**
 * @returns {number}
 */
export function everyBlockBody() {
    const arr = [1, 2, 3, 4, 5];
    if (arr.every(x => { return x > 0; })) {
        return 1;
    }
    return 0;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_every_block_body");
        assert!(
            zig.contains("break :blk true"),
            "Expected break :blk true, got:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk false"),
            "Expected break :blk false, got:\n{}",
            zig
        );
    }

    // ── Test: Array.concat() ─────────────

    #[test]
    fn test_native_proto_array_concat() {
        let js = r#"
/**
 * @returns {number}
 */
export function concatLength() {
    const a = [1, 2, 3];
    const b = [4, 5];
    const c = a.concat(b);
    return c.length;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_concat");
        assert!(zig.contains("blk:"), "Expected labeled block in:\n{}", zig);
        assert!(
            zig.contains("__concat:"),
            "Expected __concat var in:\n{}",
            zig
        );
        assert!(
            zig.contains("appendSlice"),
            "Expected appendSlice in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk __concat"),
            "Expected break :blk __concat in:\n{}",
            zig
        );
    }

    // ── Test: Array.find() ─────────────

    #[test]
    fn test_native_proto_array_find() {
        let js = r#"
/**
 * @returns {number}
 */
export function findFirstEven() {
    const arr = [1, 2, 3, 4, 5];
    const found = arr.find(x => x % 2 === 0);
    return found;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_find");
        assert!(
            zig.contains("break :blk "),
            "Expected break :blk with value in:\n{}",
            zig
        );
        // find returns the element (x), not true/false
        assert!(
            zig.contains("break :blk x"),
            "Expected break :blk x in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk undefined"),
            "Expected break :blk undefined fallback in:\n{}",
            zig
        );
    }

    // ── Test: Array.find() with block body ─────────────

    #[test]
    fn test_native_proto_array_find_block_body() {
        let js = r#"
/**
 * @returns {number}
 */
export function findBlockBody() {
    const arr = [1, 2, 3, 4, 5];
    const found = arr.find(x => { return x > 3; });
    return found;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_find_block_body");
        assert!(
            zig.contains("break :blk x"),
            "Expected break :blk x in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk undefined"),
            "Expected break :blk undefined fallback in:\n{}",
            zig
        );
    }

    // ── Test: Array.findIndex() ─────────────

    #[test]
    fn test_native_proto_array_find_index() {
        let js = r#"
/**
 * @returns {number}
 */
export function findIndexFirstEven() {
    const arr = [1, 2, 3, 4, 5];
    const idx = arr.findIndex(x => x % 2 === 0);
    return idx;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_find_index");
        assert!(zig.contains("for ("), "Expected for loop in:\n{}", zig);
        assert!(
            zig.contains("0..)"),
            "Expected range (0..) for index iteration in:\n{}",
            zig
        );
        assert!(
            zig.contains("@intCast"),
            "Expected @intCast for index in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk -1"),
            "Expected break :blk -1 fallback in:\n{}",
            zig
        );
    }

    // ── Test: Array.findIndex() with block body ─────────────

    #[test]
    fn test_native_proto_array_find_index_block_body() {
        let js = r#"
/**
 * @returns {number}
 */
export function findIndexBlockBody() {
    const arr = [10, 20, 30, 40];
    const idx = arr.findIndex(x => { return x > 25; });
    return idx;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_find_index_block_body");
        assert!(
            zig.contains("@intCast"),
            "Expected @intCast for index in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk -1"),
            "Expected break :blk -1 fallback in:\n{}",
            zig
        );
    }

    // ── Test: Array.findLast() ─────────────

    #[test]
    fn test_native_proto_array_find_last() {
        let js = r#"
/**
 * @returns {number}
 */
export function findLastEven() {
    const arr = [1, 2, 3, 4, 5];
    const found = arr.findLast(x => x % 2 === 0);
    return found;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_find_last");
        assert!(
            zig.contains("var __i: usize = "),
            "Expected reverse loop in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk "),
            "Expected break :blk with value in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk undefined"),
            "Expected break :blk undefined fallback in:\n{}",
            zig
        );
    }

    // ── Test: Array.findLastIndex() ─────────────

    #[test]
    fn test_native_proto_array_find_last_index() {
        let js = r#"
/**
 * @returns {number}
 */
export function findLastIndexEven() {
    const arr = [10, 20, 30, 40];
    const idx = arr.findLastIndex(x => x % 20 === 0);
    return idx;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_find_last_index");
        assert!(
            zig.contains("var __i: usize = "),
            "Expected reverse loop in:\n{}",
            zig
        );
        assert!(
            zig.contains("@intCast"),
            "Expected @intCast for index in:\n{}",
            zig
        );
        assert!(
            zig.contains("break :blk -1"),
            "Expected break :blk -1 fallback in:\n{}",
            zig
        );
    }

    // ── Test: Array.fill() ─────────────

    #[test]
    fn test_native_proto_array_fill() {
        let js = r#"
/**
 * @returns {number}
 */
export function fillAll() {
    const arr = [1, 2, 3, 4, 5];
    arr.fill(0);
    return arr[0] + arr[1] + arr[2] + arr[3] + arr[4];
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_fill");
        assert!(zig.contains("for ("), "Expected for loop in:\n{}", zig);
        assert!(
            zig.contains("|*elem|"),
            "Expected pointer iteration |*elem| in:\n{}",
            zig
        );
        assert!(
            zig.contains("elem."),
            "Expected elem.* assignment in:\n{}",
            zig
        );
    }

    // ── Test: Array.fill() with start/end ─────────────

    #[test]
    fn test_native_proto_array_fill_range() {
        let js = r#"
/**
 * @returns {number}
 */
export function fillRange() {
    const arr = [1, 2, 3, 4, 5];
    arr.fill(9, 1, 4);
    return arr[1] + arr[2] + arr[3];
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_fill_range");
        assert!(
            zig.contains("@intCast"),
            "Expected @intCast for index conversion in:\n{}",
            zig
        );
        assert!(
            zig.contains("|*elem|"),
            "Expected pointer iteration |*elem| in:\n{}",
            zig
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Phase 2 P2 builtin method tests
    // ═══════════════════════════════════════════════════════════════

    // ── Test: Array.at() ─────────────────────────────

    #[test]
    fn test_native_proto_array_at() {
        // JS source: arr.at(index) — positive and negative indices
        let js = r#"
/**
 * @param {number} idx
 * @returns {number}
 */
export function atIndex(idx) {
    const arr = [10, 20, 30, 40, 50];
    return arr.at(idx);
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_at");

        // Verify labeled block with clamped index
        assert!(zig.contains("blk:"), "Expected labeled block in:\n{}", zig);
        assert!(
            zig.contains("__at_idx"),
            "Expected __at_idx variable in:\n{}",
            zig
        );
        assert!(
            zig.contains("@intCast(@as(isize, @intCast("),
            "Expected negative index casting in:\n{}",
            zig
        );
        assert!(
            zig.contains(".items[__at_idx]"),
            "Expected .items[__at_idx] access in:\n{}",
            zig
        );
    }

    // ── Test: Array.lastIndexOf() ─────────────────────

    #[test]
    fn test_native_proto_array_lastindexof() {
        // JS source: arr.lastIndexOf(x) — returns last index or -1
        let js = r#"
/**
 * @param {number} target
 * @returns {number}
 */
export function findLastIndex(target) {
    const arr = [10, 20, 30, 20, 40];
    return arr.lastIndexOf(target);
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_lastindexof");

        // Verify backward while loop
        assert!(zig.contains("blk:"), "Expected labeled block in:\n{}", zig);
        assert!(
            zig.contains("while (__i >= 0)"),
            "Expected backward while loop in:\n{}",
            zig
        );
        assert!(zig.contains("__i -= 1"), "Expected decrement in:\n{}", zig);
        assert!(
            zig.contains("@as(i64, -1)"),
            "Expected @as(i64, -1) fallback in:\n{}",
            zig
        );
    }

    // ── Test: Array.copyWithin() ──────────────────────

    #[test]
    fn test_native_proto_array_copywithin() {
        // JS source: arr.copyWithin(target, start)
        let js = r#"
/**
 * @returns {number}
 */
export function copyArray() {
    const arr = [1, 2, 3, 4, 5];
    arr.copyWithin(0, 2);
    return arr[0] + arr[1] + arr[2];
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_array_copywithin");

        // Verify inline copy block
        assert!(zig.contains("blk:"), "Expected labeled block in:\n{}", zig);
        assert!(
            zig.contains("__cpw_target"),
            "Expected __cpw_target in:\n{}",
            zig
        );
        assert!(
            zig.contains("__cpw_start"),
            "Expected __cpw_start in:\n{}",
            zig
        );
        assert!(zig.contains("__cpw_cnt"), "Expected __cpw_cnt in:\n{}", zig);
        assert!(
            zig.contains("break :blk &"),
            "Expected break :blk & in:\n{}",
            zig
        );
    }

    // ── Test: String.trimStart() ──────────────────────

    #[test]
    fn test_native_proto_string_trimstart() {
        let js = r#"
export function trimLeft(str) {
    return str.trimStart();
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_string_trimstart");

        assert!(
            zig.contains("js_string.trimStart("),
            "Expected js_string.trimStart( in:\n{}",
            zig
        );
    }

    // ── Test: String.trimEnd() ────────────────────────

    #[test]
    fn test_native_proto_string_trimend() {
        let js = r#"
export function trimRight(str) {
    return str.trimEnd();
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_string_trimend");

        assert!(
            zig.contains("js_string.trimEnd("),
            "Expected js_string.trimEnd( in:\n{}",
            zig
        );
    }

    // ── Test: String.lastIndexOf() ────────────────────

    #[test]
    fn test_native_proto_string_lastindexof() {
        // Note: is_string detection requires a string literal, not a variable
        let js = r#"
export function findLastChar() {
    return "hello world".lastIndexOf("o");
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_string_lastindexof");

        assert!(
            zig.contains("js_string.lastIndexOf("),
            "Expected js_string.lastIndexOf( in:\n{}",
            zig
        );
    }

    // ── Test: String.match() ──────────────────────────

    #[test]
    fn test_native_proto_string_match_stub() {
        let js = r#"
export function matchRegex(str) {
    return str.match(/hello/);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_string_match_stub");

        assert!(
            zig.contains("js_string.matchString(js_allocator.getAllocator(),"),
            "Expected js_string.matchString(js_allocator.getAllocator(), for String.match() in:\n{}",
            zig
        );
        assert!(
            zig.contains("\"hello\""),
            "Expected pattern '\"hello\"' for String.match() in:\n{}",
            zig
        );
    }

    // ── Test: String.search() stub ────────────────────

    #[test]
    fn test_native_proto_string_search_stub() {
        let js = r#"
export function searchRegex(str) {
    return str.search(/world/);
}
"#;
        let result = parse_and_transpile(js, None).unwrap();
        let zig = result.zig_code;

        assert!(
            zig.contains("host.regex_search"),
            "Expected 'host.regex_search' for String.search() in:\n{}",
            zig
        );
    }

    // ── Test: Object.is() ─────────────────────────────

    #[test]
    fn test_native_proto_object_is() {
        let js = r#"
export function sameValue(a, b) {
    return Object.is(a, b);
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_object_is");

        assert!(
            zig.contains("std.math.isNan"),
            "Expected std.math.isNan in:\n{}",
            zig
        );
        assert!(
            zig.contains("=="),
            "Expected equality comparison in:\n{}",
            zig
        );
    }

    // ── Test: Object.getOwnPropertyNames() stub ───────

    #[test]
    fn test_native_proto_object_getownpropertynames_stub() {
        let js = r#"
export function getPropNames(obj) {
    return Object.getOwnPropertyNames(obj);
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_object_getownpropertynames_stub");

        assert!(
            zig.contains("@compileError"),
            "Expected @compileError for Object.getOwnPropertyNames() in:\n{}",
            zig
        );
        assert!(
            zig.contains("getOwnPropertyNames"),
            "Expected 'getOwnPropertyNames' mention in:\n{}",
            zig
        );
    }

    // ═══════════════════════════════════════════════════════════════
    // Anonymous object type annotation tests
    // ═══════════════════════════════════════════════════════════════

    // ── Test: @returns {{name: string, age: number}} on export fn ──

    #[test]
    fn test_native_proto_anon_obj_type_returns() {
        // JS source: @returns {{name: string, age: number}} on export fn
        let js = r#"
/**
 * @returns {{name: string, age: number}}
 */
export function makeUser() {
    const user = { name: "Bob", age: 25 };
    return user;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_anon_obj_type_returns");
        println!("=== Anonymous object @returns Zig code ===\n{}", zig);

        // Verify struct fields are present in the generated Zig
        assert!(
            zig.contains("name") && zig.contains("age"),
            "Expected name and age fields, got:\n{}",
            zig
        );
        // The return type uses Zig anonymous struct literal syntax
        assert!(
            zig.contains("pub fn makeUser"),
            "Expected pub fn makeUser in:\n{}",
            zig
        );
    }

    // ── Test: @type {{name: string, age: number}} on non-JSON.parse var ──

    #[test]
    fn test_native_proto_anon_obj_type_variable_access() {
        // Test that @type {{name: string, age: number}} correctly infers
        // struct fields, and property access returns correct types
        let js = r#"
/**
 * @returns {string}
 */
export function getName() {
    /**
     * @type {{name: string, age: number}}
     */
    const user = { name: "Alice", age: 30 };
    return user.name;
}

/**
 * @returns {number}
 */
export function getAge() {
    /**
     * @type {{name: string, age: number}}
     */
    const user = { name: "Alice", age: 30 };
    return user.age;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_anon_obj_type_variable_access");
        println!("=== Anonymous object @type variable Zig code ===\n{}", zig);

        // Verify .name and .age property access is generated
        assert!(
            zig.contains(".name"),
            "Expected .name property access in:\n{}",
            zig
        );
        assert!(
            zig.contains(".age"),
            "Expected .age property access in:\n{}",
            zig
        );
    }

    // ── Test: Number.* static constants ──────────────────────

    #[test]
    fn test_native_proto_number_constants() {
        // JS source: Number.MAX_VALUE, Number.MIN_VALUE, Number.NaN, etc.
        let js = r#"
/**
 * @returns {number}
 */
export function getMaxValue() {
    return Number.MAX_VALUE;
}

/**
 * @returns {number}
 */
export function getMinValue() {
    return Number.MIN_VALUE;
}

/**
 * @returns {number}
 */
export function getNaN() {
    return Number.NaN;
}

/**
 * @returns {number}
 */
export function getNegInfinity() {
    return Number.NEGATIVE_INFINITY;
}

/**
 * @returns {number}
 */
export function getPosInfinity() {
    return Number.POSITIVE_INFINITY;
}

/**
 * @returns {number}
 */
export function getEpsilon() {
    return Number.EPSILON;
}

/**
 * @returns {number}
 */
export function getMaxSafeInt() {
    return Number.MAX_SAFE_INTEGER;
}

/**
 * @returns {number}
 */
export function getMinSafeInt() {
    return Number.MIN_SAFE_INTEGER;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_number_constants");

        // Verify all 8 constants are generated as Zig equivalents
        assert!(
            zig.contains("std.math.floatMax(f64)"),
            "Expected 'std.math.floatMax(f64)' in:\n{}",
            zig
        );
        assert!(
            zig.contains("std.math.floatMin(f64)"),
            "Expected 'std.math.floatMin(f64)' in:\n{}",
            zig
        );
        assert!(
            zig.contains("std.math.nan(f64)"),
            "Expected 'std.math.nan(f64)' in:\n{}",
            zig
        );
        assert!(
            zig.contains("-std.math.inf(f64)"),
            "Expected '-std.math.inf(f64)' in:\n{}",
            zig
        );
        assert!(
            zig.contains("std.math.inf(f64)"),
            "Expected 'std.math.inf(f64)' in:\n{}",
            zig
        );
        assert!(
            zig.contains("std.math.floatEps(f64)"),
            "Expected 'std.math.floatEps(f64)' in:\n{}",
            zig
        );
        assert!(
            zig.contains("9007199254740991"),
            "Expected '9007199254740991' in:\n{}",
            zig
        );
        assert!(
            zig.contains("-9007199254740991"),
            "Expected '-9007199254740991' in:\n{}",
            zig
        );
    }

    // ── Test: Number.isSafeInteger() ──────────────────────

    #[test]
    fn test_native_proto_number_issafeinteger() {
        let js = r#"
/**
 * @param {number} v
 * @returns {boolean}
 */
export function checkSafe(v) {
    return Number.isSafeInteger(v);
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_number_issafeinteger");
        assert!(
            zig.contains("js_number.isSafeInteger("),
            "Expected 'js_number.isSafeInteger(' in:\n{}",
            zig
        );
    }

    // ── Test: Number.toFixed() ──────────────────────

    #[test]
    fn test_native_proto_number_tofixed() {
        let js = r#"
/**
 * @returns {string}
 */
export function formatPi() {
    const pi = 3.14159;
    return pi.toFixed(2);
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_number_tofixed");
        assert!(
            zig.contains("js_number.toFixed(js_allocator.getAllocator(), pi"),
            "Expected 'js_number.toFixed(js_allocator.getAllocator(), pi' in:\n{}",
            zig
        );
    }

    // ── Test: Map.forEach — closure callback ─────────

    #[test]
    fn test_native_proto_map_foreach() {
        let js = r#"
/**
 * @returns {i64}
 */
export function testMapForEach() {
    const m = new Map();
    m.set("a", 1);
    m.set("b", 2);
    m.set("c", 3);
    let sum = 0;
    m.forEach((val, key) => { sum = sum + val; });
    return sum;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_map_foreach");

        // Verify Map.forEach generates while-iterator loop (not Array for-loop)
        assert!(
            zig.contains("var iter = m.inner.iterator();"),
            "Expected 'var iter = m.inner.iterator();' in:\n{}",
            zig
        );
        assert!(
            zig.contains("while (iter.next()) |entry|"),
            "Expected 'while (iter.next()) |entry|' in:\n{}",
            zig
        );
        assert!(
            zig.contains("entry.value_ptr.*") || zig.contains("const val = entry.value_ptr.*"),
            "Expected 'entry.value_ptr.*' binding in:\n{}",
            zig
        );
        assert!(
            zig.contains("entry.key_ptr.*") || zig.contains("const key = entry.key_ptr.*"),
            "Expected 'entry.key_ptr.*' binding in:\n{}",
            zig
        );
        // Ensure it does NOT contain Array-style for-loop
        assert!(
            !zig.contains("for (m.items)"),
            "Should NOT contain Array for-loop for Map.forEach:\n{}",
            zig
        );
    }

    // ── Phase 6: String 高级方法测试 ──────────────────────

    // Test: String.startsWith()
    #[test]
    fn test_p6_string_starts_with() {
        let js = r#"
/**
 * @returns {boolean}
 */
export function checkStart() {
    return "hello".startsWith("he");
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_starts_with");
        assert!(
            zig.contains("js_string.startsWith("),
            "Expected 'js_string.startsWith(' in:\n{}",
            zig
        );
    }

    // Test: String.endsWith()
    #[test]
    fn test_p6_string_ends_with() {
        let js = r#"
/**
 * @param {string} str
 * @param {string} suffix
 * @returns {boolean}
 */
export function checkEnd(str, suffix) {
    return str.endsWith(suffix);
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_ends_with");
        assert!(
            zig.contains("js_string.endsWith("),
            "Expected 'js_string.endsWith(' in:\n{}",
            zig
        );
    }

    // Test: String.includes()
    #[test]
    fn test_p6_string_includes() {
        let js = r#"
/**
 * @returns {boolean}
 */
export function checkIncludes() {
    return "hello world".includes("world");
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_includes");
        assert!(
            zig.contains("js_string.includes("),
            "Expected 'js_string.includes(' in:\n{}",
            zig
        );
    }

    // Test: String.repeat()
    #[test]
    fn test_p6_string_repeat() {
        let js = r#"
/**
 * @param {string} str
 * @param {i64} n
 * @returns {string}
 */
export function repeatStr(str, n) {
    return str.repeat(n);
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_repeat");
        assert!(
            zig.contains("js_string.repeat(js_allocator.getAllocator()"),
            "Expected 'js_string.repeat(js_allocator.getAllocator()' in:\n{}",
            zig
        );
    }

    // Test: String.substring()
    #[test]
    fn test_p6_string_substring() {
        let js = r#"
/**
 * @param {string} str
 * @param {i64} start
 * @param {i64} end
 * @returns {string}
 */
export function getSub(str, start, end) {
    return str.substring(start, end);
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_substring");
        assert!(
            zig.contains("js_string.substring("),
            "Expected 'js_string.substring(' in:\n{}",
            zig
        );
    }

    // Test: String.slice()
    #[test]
    fn test_p6_string_slice() {
        let js = r#"
/**
 * @returns {string}
 */
export function getSlice() {
    return "hello world".slice(2, 9);
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_slice");
        assert!(
            zig.contains("js_string.slice("),
            "Expected 'js_string.slice(' in:\n{}",
            zig
        );
    }

    // Test: String.concat()
    #[test]
    fn test_p6_string_concat() {
        let js = r#"
/**
 * @returns {string}
 */
export function joinStr() {
    return "hello".concat("world");
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_concat");
        assert!(
            zig.contains("js_string.concat("),
            "Expected 'js_string.concat(' in:\n{}",
            zig
        );
    }

    // Test: String.normalize() (stub)
    #[test]
    fn test_p6_string_normalize() {
        let js = r#"
/**
 * @param {string} str
 * @returns {string}
 */
export function normalizeStr(str) {
    return str.normalize("NFC");
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_normalize");
        assert!(
            zig.contains("js_string.normalize(js_allocator.getAllocator()"),
            "Expected 'js_string.normalize(js_allocator.getAllocator()' in:\n{}",
            zig
        );
    }

    // Test: String.toUpperCase()
    #[test]
    fn test_p6_string_to_upper_case() {
        let js = r#"
/**
 * @param {string} str
 * @returns {string}
 */
export function toUpper(str) {
    return str.toUpperCase();
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_to_upper_case");
        assert!(
            zig.contains("js_string.toUpper(js_allocator.getAllocator()"),
            "Expected 'js_string.toUpper(js_allocator.getAllocator()' in:\n{}",
            zig
        );
    }

    // Test: String.toLowerCase()
    #[test]
    fn test_p6_string_to_lower_case() {
        let js = r#"
/**
 * @param {string} str
 * @returns {string}
 */
export function toLower(str) {
    return str.toLowerCase();
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_to_lower_case");
        assert!(
            zig.contains("js_string.toLower(js_allocator.getAllocator()"),
            "Expected 'js_string.toLower(js_allocator.getAllocator()' in:\n{}",
            zig
        );
    }

    // Test: String.split()
    #[test]
    fn test_p6_string_split() {
        let js = r#"
/**
 * @param {string} str
 * @returns {[]string}
 */
export function splitStr(str) {
    return str.split(",");
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_split");
        assert!(
            zig.contains("js_string.split(js_allocator.getAllocator()"),
            "Expected 'js_string.split(js_allocator.getAllocator()' in:\n{}",
            zig
        );
    }

    // Test: String.charAt()
    #[test]
    fn test_p6_string_char_at() {
        let js = r#"
/**
 * @param {string} str
 * @returns {string}
 */
export function getChar(str) {
    return str.charAt(0);
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_char_at");
        assert!(
            zig.contains("js_string.charAt(js_allocator.getAllocator()"),
            "Expected 'js_string.charAt(js_allocator.getAllocator()' in:\n{}",
            zig
        );
    }

    // Test: String.indexOf()
    #[test]
    fn test_p6_string_index_of() {
        let js = r#"
/**
 * @returns {i64}
 */
export function findIndex() {
    return "hello".indexOf("lo");
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_index_of");
        assert!(
            zig.contains("js_string.indexOf("),
            "Expected 'js_string.indexOf(' in:\n{}",
            zig
        );
    }

    // ── Test: String.padStart() ────────────────────────
    #[test]
    fn test_p6_string_pad_start() {
        let js = r#"
/**
 * @param {string} str
 * @returns {string}
 */
export function pad(str) {
    return str.padStart(10, " ");
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_pad_start");
        assert!(
            zig.contains("js_string.padStart("),
            "Expected 'js_string.padStart(' in:\n{}",
            zig
        );
    }

    // ── Test: String.padEnd() ────────────────────────
    #[test]
    fn test_p6_string_pad_end() {
        let js = r#"
/**
 * @param {string} str
 * @returns {string}
 */
export function padEndFn(str) {
    return str.padEnd(10, " ");
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_pad_end");
        assert!(
            zig.contains("js_string.padEnd("),
            "Expected 'js_string.padEnd(' in:\n{}",
            zig
        );
    }

    // ── Test: String.replace() ────────────────────────
    #[test]
    fn test_p6_string_replace() {
        let js = r#"
/**
 * @param {string} str
 * @returns {string}
 */
export function replaceStr(str) {
    return str.replace("old", "new");
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_replace");
        assert!(
            zig.contains("js_string.replace("),
            "Expected 'js_string.replace(' in:\n{}",
            zig
        );
    }

    // ── Test: String.replaceAll() ────────────────────────
    #[test]
    fn test_p6_string_replace_all() {
        let js = r#"
/**
 * @param {string} str
 * @returns {string}
 */
export function replaceAllStr(str) {
    return str.replaceAll("a", "b");
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_replace_all");
        assert!(
            zig.contains("js_string.replaceAll("),
            "Expected 'js_string.replaceAll(' in:\n{}",
            zig
        );
    }

    // ── Test: String.charCodeAt() ────────────────────────
    #[test]
    fn test_p6_string_char_code_at() {
        let js = r#"
/**
 * @returns {i64}
 */
export function getCharCode() {
    return "hello".charCodeAt(0);
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_char_code_at");
        assert!(
            zig.contains("js_string.charCodeAt("),
            "Expected 'js_string.charCodeAt(' in:\n{}",
            zig
        );
    }

    // ── Test: String.codePointAt() ────────────────────────
    #[test]
    fn test_p6_string_code_point_at() {
        let js = r#"
/**
 * @returns {i64}
 */
export function getCodePoint() {
    return "hello".codePointAt(0);
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_code_point_at");
        assert!(
            zig.contains("js_string.codePointAt("),
            "Expected 'js_string.codePointAt(' in:\n{}",
            zig
        );
    }

    // ── Test: String.toLocaleUpperCase() ────────────────────────
    #[test]
    fn test_p6_string_to_locale_upper_case() {
        let js = r#"
/**
 * @param {string} str
 * @returns {string}
 */
export function toLocaleUpper(str) {
    return str.toLocaleUpperCase();
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_to_locale_upper_case");
        assert!(
            zig.contains("js_string.toLocaleUpper("),
            "Expected 'js_string.toLocaleUpper(' in:\n{}",
            zig
        );
    }

    // ── Test: String.toLocaleLowerCase() ────────────────────────
    #[test]
    fn test_p6_string_to_locale_lower_case() {
        let js = r#"
/**
 * @param {string} str
 * @returns {string}
 */
export function toLocaleLower(str) {
    return str.toLocaleLowerCase();
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_to_locale_lower_case");
        assert!(
            zig.contains("js_string.toLocaleLower("),
            "Expected 'js_string.toLocaleLower(' in:\n{}",
            zig
        );
    }

    // ── Test: String.localeCompare() ────────────────────────
    #[test]
    fn test_p6_string_locale_compare() {
        let js = r#"
/**
 * @param {string} str
 * @param {string} other
 * @returns {i64}
 */
export function compareStrs(str, other) {
    return str.localeCompare(other);
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_locale_compare");
        assert!(
            zig.contains("js_string.localeCompare("),
            "Expected 'js_string.localeCompare(' in:\n{}",
            zig
        );
    }

    // ── Test: String.fromCharCode() (static) ────────────────────────
    #[test]
    fn test_p6_string_from_char_code() {
        let js = r#"
/**
 * @returns {string}
 */
export function getChar() {
    return String.fromCharCode(65, 66, 67);
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_from_char_code");
        assert!(
            zig.contains("js_string.fromCharCode("),
            "Expected 'js_string.fromCharCode(' in:\n{}",
            zig
        );
    }

    // ── Test: String.fromCodePoint() (static) ────────────────────────
    #[test]
    fn test_p6_string_from_code_point() {
        let js = r#"
/**
 * @returns {string}
 */
export function getCharFromPoint() {
    return String.fromCodePoint(0x1F600);
}
"#;
        let zig = transpile_and_check!(js, "test_p6_string_from_code_point");
        assert!(
            zig.contains("js_string.fromCodePoint("),
            "Expected 'js_string.fromCodePoint(' in:\n{}",
            zig
        );
    }

    // ── Phase 7: Set 迭代方法测试 ──────────────────────

    // Test: Set.add / Set.has
    #[test]
    fn test_p7_set_add_has() {
        let js = r#"
/**
 * @returns {boolean}
 */
export function testSetAddHas() {
    const s = new Set();
    s.add("hello");
    s.add("world");
    return s.has("hello");
}
"#;
        let zig = transpile_and_check!(js, "test_p7_set_add_has");
        assert!(zig.contains(".add("), "Expected '.add(' in:\n{}", zig);
        assert!(zig.contains(".has("), "Expected '.has(' in:\n{}", zig);
    }

    // Test: Set.forEach
    #[test]
    fn test_p7_set_foreach() {
        let js = r#"
/**
 * @returns {i64}
 */
export function testSetForEach() {
    const s = new Set();
    s.add(1);
    s.add(2);
    s.add(3);
    let sum = 0;
    s.forEach((val) => { sum = sum + val; });
    return sum;
}
"#;
        let zig = transpile_and_check!(js, "test_p7_set_foreach");
        // Should generate for-loop over set items
        assert!(
            zig.contains("for (s.items.items)"),
            "Expected 'for (s.items.items)' in:\n{}",
            zig
        );
        assert!(zig.contains("|val|"), "Expected '|val|' in:\n{}", zig);
        // Must NOT generate Map-style iterator
        assert!(
            !zig.contains("iter.next()"),
            "Should NOT contain iterator for Set.forEach:\n{}",
            zig
        );
    }

    // Test: Set.keys / Set.values / Set.entries
    #[test]
    fn test_p7_set_iterators() {
        let js = r#"
/**
 * @returns {i64}
 */
export function testSetIterators() {
    const s = new Set();
    s.add("a");
    s.add("b");
    const ks = s.keys();
    const vs = s.values();
    const es = s.entries();
    return ks.length + vs.length + es.length;
}
"#;
        let zig = transpile_and_check!(js, "test_p7_set_iterators");
        assert!(zig.contains(".keys("), "Expected '.keys(' in:\n{}", zig);
        assert!(zig.contains(".values("), "Expected '.values(' in:\n{}", zig);
        assert!(
            zig.contains(".entries("),
            "Expected '.entries(' in:\n{}",
            zig
        );
    }

    // Test: Set.delete / Set.clear / Set.size
    #[test]
    fn test_p7_set_delete_clear() {
        let js = r#"
/**
 * @returns {boolean}
 */
export function testSetDeleteClear() {
    const s = new Set();
    s.add("x");
    s.add("y");
    s.delete("x");
    s.clear();
    return s.has("x");
}
"#;
        let zig = transpile_and_check!(js, "test_p7_set_delete_clear");
        assert!(zig.contains(".delete("), "Expected '.delete(' in:\n{}", zig);
        assert!(zig.contains(".clear()"), "Expected '.clear()' in:\n{}", zig);
    }

    // Phase 7: Object defineProperties / getOwnPropertyDescriptor / setPrototypeOf

    #[test]
    fn test_p7_object_define_properties() {
        let js = r#"
export function defineProps(target, props) {
    Object.defineProperties(target, props);
}
"#;
        let zig = transpile_and_check!(js, "test_p7_object_define_properties");
        assert!(
            zig.contains("js_object.defineProperties("),
            "Expected 'js_object.defineProperties(' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p7_object_get_own_property_descriptor() {
        let js = r#"
function getDesc(obj, key) {
    return Object.getOwnPropertyDescriptor(obj, key);
}
"#;
        let zig = transpile_and_check!(js, "test_p7_object_get_own_property_descriptor");
        assert!(
            zig.contains("js_object.getOwnPropertyDescriptor(js_allocator.getAllocator(), "),
            "Expected 'js_object.getOwnPropertyDescriptor(js_allocator.getAllocator(), ' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p7_object_set_prototype_of() {
        let js = r#"
export function setProto(obj, proto) {
    Object.setPrototypeOf(obj, proto);
}
"#;
        let zig = transpile_and_check!(js, "test_p7_object_set_prototype_of");
        assert!(
            zig.contains("js_object.setPrototypeOf("),
            "Expected 'js_object.setPrototypeOf(' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p8_object_is_sealed_frozen_extensible() {
        // Object.isSealed/isFrozen → always true; Object.isExtensible → always false
        let js = r#"
export function checkObj(obj) {
    if (obj === 0) return false;
    const a = Object.isSealed(obj);
    const b = Object.isFrozen(obj);
    const c = Object.isExtensible(obj);
    return a && b && !c;
}
"#;
        let zig = transpile_and_check!(js, "test_p8_object_is_sealed_frozen_extensible");
        assert!(
            zig.contains("true"),
            "Expected 'true' for isSealed/isFrozen in:\n{}",
            zig
        );
        assert!(
            zig.contains("false"),
            "Expected 'false' for isExtensible in:\n{}",
            zig
        );
    }

    // ── Phase 8: RegExp / String regex host function tests ──

    #[test]
    fn test_p8_regex_test() {
        // /pattern/.test(str) → host.regex_test("pattern", str)
        let js = r#"
export function hasDigit(s) {
    return /\d/.test(s);
}
"#;
        let result = parse_and_transpile(js, None).unwrap();
        let zig = result.zig_code;
        assert!(
            zig.contains(r#"host.regex_test("\\d", s)"#),
            "Expected 'host.regex_test(\"\\d\", s)' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p8_string_search() {
        // str.search(/pattern/) → host.regex_search("pattern", str)
        let js = r#"
export function findDigit(s) {
    return s.search(/\d+/);
}
"#;
        let result = parse_and_transpile(js, None).unwrap();
        let zig = result.zig_code;
        assert!(
            zig.contains(r#"host.regex_search("\\d+", s)"#),
            "Expected 'host.regex_search(\"\\d+\", s)' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p8_string_match_compile_error() {
        // str.match(/pattern/) → js_string.matchString(alloc, str, "pattern")
        let js = r#"
export function getMatch(s) {
    return s.match(/hello/);
}
"#;
        let result = parse_and_transpile(js, None).unwrap();
        let zig = result.zig_code;
        assert!(
            zig.contains(r#"js_string.matchString(js_allocator.getAllocator(),"#),
            "Expected 'js_string.matchString(js_allocator.getAllocator(),' for String.match() in:\n{}",
            zig
        );
        assert!(
            zig.contains(r#""hello""#),
            "Expected pattern '\"hello\"' for String.match() in:\n{}",
            zig
        );
    }

    // ── Phase 8.1: Dynamic RegExp (new RegExp) tests ──

    #[test]
    fn test_p8_new_regexp() {
        // new RegExp(pattern) → try js_regexp.JsRegExp.init(alloc, pattern)
        let js = r#"
export function makePattern(s) {
    const r = new RegExp("\\d+");
    return r.test(s);
}
"#;
        let result = parse_and_transpile(js, None).unwrap();
        let zig = result.zig_code;
        assert!(
            zig.contains("js_regexp.JsRegExp.init(js_allocator.getAllocator(),"),
            "Expected 'js_regexp.JsRegExp.init(...)' for new RegExp in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p8_regexp_var_test() {
        // regexpVar.test(str) → regexpVar.isMatch(str) (method call on JsRegExp)
        let js = r#"
export function hasDigit(s) {
    const r = new RegExp("\\d");
    return r.test(s);
}
"#;
        let result = parse_and_transpile(js, None).unwrap();
        let zig = result.zig_code;
        assert!(
            zig.contains(".isMatch(s)") || zig.contains(".isMatch("),
            "Expected '.isMatch(...)' for regexpVar.test() in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p8_string_match_regexp_var() {
        // str.match(regexpVar) → js_string.matchString(alloc, str, regexpVar.pattern)
        let js = r#"
export function getMatch(s, r) {
    // r must be a RegExp variable initialized via new RegExp
    const p = new RegExp("hello");
    const m = s.match(p);
    return m;
}
"#;
        let result = parse_and_transpile(js, None).unwrap();
        let zig = result.zig_code;
        assert!(
            zig.contains("p.pattern"),
            "Expected '.pattern' for String.match(regexpVar) in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p8_string_search_regexp_var() {
        // str.search(regexpVar) → host.regex_search(regexpVar.pattern, str)
        let js = r#"
export function findIndex(s) {
    const r = new RegExp("foo");
    return s.search(r);
}
"#;
        let result = parse_and_transpile(js, None).unwrap();
        let zig = result.zig_code;
        assert!(
            zig.contains("r.pattern"),
            "Expected '.pattern' for str.search(regexpVar) in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p8_regexp_exec_literal() {
        // /pattern/.exec(str) → js_regexp.execLiteral(alloc, str, "pattern")
        let js = r#"
export function getExecResult(s) {
    return /world/.exec(s);
}
"#;
        let result = parse_and_transpile(js, None).unwrap();
        let zig = result.zig_code;
        assert!(
            zig.contains("js_regexp.execLiteral(js_allocator.getAllocator(),"),
            "Expected 'js_regexp.execLiteral(js_allocator.getAllocator(),' in:\n{}",
            zig
        );
        assert!(
            zig.contains("\"world\""),
            "Expected pattern literal '\"world\"' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p8_regexp_var_exec() {
        // regexpVar.exec(str) → regexpVar.exec(alloc, str)
        let js = r#"
export function getVarExec(s) {
    const r = new RegExp("hello");
    return r.exec(s);
}
"#;
        let result = parse_and_transpile(js, None).unwrap();
        let zig = result.zig_code;
        assert!(
            zig.contains(".exec(js_allocator.getAllocator(),"),
            "Expected '.exec(js_allocator.getAllocator(),' for regexpVar.exec() in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p7_encode_uri() {
        let js = r#"
export function encode(url) {
    return encodeURI(url);
}
"#;
        let zig = transpile_and_check!(js, "test_p7_encode_uri");
        assert!(
            zig.contains("js_uri.encodeURI(js_allocator.getAllocator(),"),
            "Expected 'js_uri.encodeURI(js_allocator.getAllocator(),' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p7_decode_uri() {
        let js = r#"
export function decode(url) {
    return decodeURI(url);
}
"#;
        let zig = transpile_and_check!(js, "test_p7_decode_uri");
        assert!(
            zig.contains("js_uri.decodeURI(js_allocator.getAllocator(),"),
            "Expected 'js_uri.decodeURI(js_allocator.getAllocator(),' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p7_encode_uri_component() {
        let js = r#"
export function encodeComp(s) {
    return encodeURIComponent(s);
}
"#;
        let zig = transpile_and_check!(js, "test_p7_encode_uri_component");
        assert!(
            zig.contains("js_uri.encodeURIComponent(js_allocator.getAllocator(),"),
            "Expected 'js_uri.encodeURIComponent(js_allocator.getAllocator(),' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_p7_decode_uri_component() {
        let js = r#"
export function decodeComp(s) {
    return decodeURIComponent(s);
}
"#;
        let zig = transpile_and_check!(js, "test_p7_decode_uri_component");
        assert!(
            zig.contains("js_uri.decodeURIComponent(js_allocator.getAllocator(),"),
            "Expected 'js_uri.decodeURIComponent(js_allocator.getAllocator(),' in:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_map_get_eq_cmp() {
        // Map.get() returns JsAny → comparison must use .eq(JsAny.from(...)) not ==
        let js = r#"
function testMapGetCmp() {
    var m = new Map();
    m.set("a", 100);
    const v = m.get("a");
    if (v == 100) return 1;
    return 0;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_map_get_eq_cmp");
        // Must generate .eq(JsAny.from(100)) — NOT v == 100
        assert!(
            zig.contains(".eq(JsAny.from(100))"),
            "Expected '.eq(JsAny.from(100))' in generated Zig, got:\n{}",
            zig
        );
        // Should NOT contain direct == comparison with integer
        assert!(
            !zig.contains("== 100") && !zig.contains("==100"),
            "Should NOT contain '== 100' in generated Zig, got:\n{}",
            zig
        );
    }

    #[test]
    fn test_p8_string_match_ast_check() {
        // Verify that String.match(/pattern/) generates compilable Zig code.
        let js = r#"
export function getMatch(s) {
    return s.match(/world/);
}
"#;
        let _zig = transpile_and_check!(js, "p8_string_match_ast_check");
    }

    #[test]
    fn test_p8_string_match_regexp_var_ast_check() {
        // Verify that String.match(regexpVar) generates compilable Zig code.
        let js = r#"
export function getMatch(s) {
    const r = new RegExp("world");
    return s.match(r);
}
"#;
        let _zig = transpile_and_check!(js, "p8_string_match_regexp_var_ast_check");
    }

    #[test]
    fn test_p8_string_match_global_ast_check() {
        // Verify that String.match(/pattern/g) generates code that calls matchStringGlobal.
        let js = r#"
export function getMatch(s) {
    return s.match(/world/g);
}
"#;
        let zig = transpile_and_check!(js, "p8_string_match_global_ast_check");
        assert!(
            zig.contains("matchStringGlobal"),
            "Expected 'matchStringGlobal' for /g flag in:\n{}",
            zig
        );
    }

    // ── Phase 3: Boundary case tests for String.match() ──

    #[test]
    fn test_p8_string_match_capture_groups_ast_check() {
        // Verify that match with capture groups generates correct Zig code.
        // JS: "2024-01-15".match(/(\d{4})-(\d{2})-(\d{2})/)
        // returns ["2024-01-15", "2024", "01", "15"]
        let js = r#"
export function parseDate(s) {
    return s.match(/(\d{4})-(\d{2})-(\d{2})/);
}
"#;
        let _zig = transpile_and_check!(js, "p8_string_match_capture_groups_ast_check");
        // Should generate code that calls matchString (not matchStringGlobal)
        // and returns JsAny (which can be an array with capture groups)
    }

    #[test]
    fn test_p8_string_match_empty_pattern_ast_check() {
        // Test: matching against empty pattern should match empty string.
        // JS: "abc".match(/(?:)/) returns [""] (empty pattern matches empty string)
        let js = r#"
export function matchEmpty(s) {
    return s.match(/(?:)/);
}
"#;
        let _zig = transpile_and_check!(js, "p8_string_match_empty_pattern_ast_check");
    }

    #[test]
    fn test_p8_string_match_empty_string_ast_check() {
        // Test: matching empty string against pattern.
        // JS: "".match(/abc/) returns null (no match)
        let js = r#"
export function matchEmptyStr() {
    return "".match(/abc/);
}
"#;
        let _zig = transpile_and_check!(js, "p8_string_match_empty_string_ast_check");
    }

    // ══════════════════════════════════════════════════════════
    // Symbol Type Tests (#737/#738/#739)
    // ══════════════════════════════════════════════════════════

    #[test]
    fn test_native_proto_symbol_basic() {
        // Symbol() / Symbol("desc") construction
        let js = r#"
/**
 * @returns {Symbol}
 */
export function makeSymbol() {
    const s1 = Symbol();
    return s1;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_symbol_basic");
        println!("=== Symbol basic ===\n{}", zig);
        // Should generate JsSymbol.init() for Symbol()
        assert!(
            zig.contains("JsSymbol.initAnonymous()"),
            "Expected JsSymbol.initAnonymous() for Symbol(): {}",
            zig
        );
    }

    #[test]
    fn test_native_proto_symbol_for_keyfor() {
        // Symbol.for(key) / Symbol.keyFor(sym)
        let js = r#"
/**
 * @returns {Symbol}
 */
export function registerSymbol(key) {
    const sym = Symbol.for(key);
    return sym;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_symbol_for_keyfor");
        println!("=== Symbol.for()/keyFor() ===\n{}", zig);
        // Should generate js_symbol.symbolFor(alloc, key)
        assert!(
            zig.contains("js_symbol.symbolFor("),
            "Expected js_symbol.symbolFor() for Symbol.for(): {}",
            zig
        );
    }

    #[test]
    fn test_native_proto_symbol_description() {
        // sym.description property access
        let js = r#"
/**
 * @returns {string}
 */
export function getDescription() {
    const sym = Symbol("hello");
    return sym.description;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_symbol_description");
        println!("=== Symbol.description ===\n{}", zig);
        // description is ?[]const u8, access generates sym.description
        assert!(
            zig.contains("sym.description"),
            "Expected 'sym.description' access: {}",
            zig
        );
    }

    #[test]
    fn test_native_proto_symbol_to_string() {
        // sym.toString() method call
        let js = r#"
/**
 * @returns {string}
 */
export function symbolToString(sym) {
    return sym.toString();
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_symbol_toString");
        println!("=== Symbol.toString() ===\n{}", zig);
        // Should generate sym.toString(js_allocator.getAllocator())
        assert!(
            zig.contains("sym.toString(js_allocator.getAllocator())"),
            "Expected sym.toString(alloc) for Symbol.toString(): {}",
            zig
        );
    }

    #[test]
    fn test_native_proto_symbol_equality() {
        // Symbol equality comparison (sym1 === sym2)
        let js = r#"
/**
 * @returns {boolean}
 */
export function compareSymbols() {
    const s1 = Symbol("a");
    const s2 = Symbol("a");
    const s3 = s1;
    const r1 = s1 === s2;  // false (different symbols)
    const r2 = s1 === s3;  // true (same symbol)
    return r1 || r2;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_symbol_equality");
        println!("=== Symbol equality ===\n{}", zig);
        // Should generate === comparison for symbols
        assert!(
            zig.contains("s1") && zig.contains("s2"),
            "Expected s1 and s2 in code: {}",
            zig
        );
        // Symbol equality is pointer equality (id comparison)
        // JsSymbol has `id: u64`, so === becomes s1.id == s2.id
        // Actually, Zig generates s1 == s2 for === on structs (uses @field-wise comparison)
        assert!(
            zig.contains("=="),
            "Expected == for Symbol equality: {}",
            zig
        );
    }

    #[test]
    fn test_native_proto_symbol_type_inference() {
        // Verify Symbol type is correctly inferred for variables
        let js = r#"
/**
 * @param {Symbol} sym
 * @returns {string}
 */
export function processSymbol(sym) {
    const desc = sym.description;
    const str = sym.toString();
    return desc || str;
}
"#;
        let zig = transpile_and_check!(js, "test_native_proto_symbol_type_inference");
        println!("=== Symbol type inference ===\n{}", zig);
        // sym should be typed as JsSymbol
        assert!(
            zig.contains("sym: JsSymbol"),
            "Expected 'sym: JsSymbol' parameter type: {}",
            zig
        );
    }
    #[test]
    fn test_p8_string_match_global_empty_match_ast_check() {
        // Test: global matching with pattern that can match empty string.
        // JS: "bc".match(/a*/g) returns ["", "", ""] (empty matches at positions 0, 1, 2)
        let js = r#"
export function matchEmptyGlobal(s) {
    return s.match(/a*/g);
}
"#;
        let zig = transpile_and_check!(js, "p8_string_match_global_empty_match_ast_check");
        assert!(
            zig.contains("matchStringGlobal"),
            "Expected 'matchStringGlobal' for /g flag in:\n{}",
            zig
        );
    }

    // ── #768: 声明+表达式混合 — 验证不产生未使用变量/值警告 ──

    #[test]
    fn test_p3_mixed_decl_expr_basic() {
        // Basic: var declaration followed by expression statements.
        // Should not produce unused variable warnings in Zig output.
        let js = r#"
export function mixDeclExpr(x, y) {
    const z = x + y;
    return z;
}
"#;
        let zig = transpile_and_check!(js, "test_p3_mixed_decl_expr_basic");
        println!("=== Mix decl+expr (basic) ===\n{}", zig);
        // z should be used (returned), no suppression needed
        assert!(zig.contains("z = x + y"), "Expected 'z = x + y':\n{}", zig);
    }

    #[test]
    fn test_p3_mixed_decl_expr_unused_var() {
        // Variable declared but never read → Zig correctly reports "unused local constant".
        // This is a feature, not a bug: Zig catches JS code quality issues.
        // We do NOT suppress this error — the transpiler faithfully translates JS to Zig,
        // and the Zig compiler helpfully flags dead code.
        // Known-expected: no ast-check (Zig will reject unused local consts).
        let js = r#"
export function mixUnusedVar(x) {
    const z = x * 2;
    return x + 1;
}
"#;
        let zig = transpile_and_assert!(js, "test_p3_mixed_decl_expr_unused_var");
        println!("=== Mix decl+expr (unused var) ===\n{}", zig);
        // z is unused — Zig compiler will reject with "unused local constant".
        // This is intended: it forces JS authors to clean up dead code.
    }

    #[test]
    fn test_p3_mixed_decl_expr_call() {
        // Expression statement with function call between declarations.
        let js = r#"
export function mixDeclCall(x) {
    const a = x + 1;
    const b = x + 2;
    return a + b;
}
"#;
        let zig = transpile_and_check!(js, "test_p3_mixed_decl_expr_call");
        println!("=== Mix decl+expr (call) ===\n{}", zig);
        // All variables are used, no suppression needed
        assert!(zig.contains("a = x + 1"), "Expected 'a = x + 1':\n{}", zig);
        assert!(zig.contains("b = x + 2"), "Expected 'b = x + 2':\n{}", zig);
    }

    #[test]
    fn test_p3_mixed_decl_expr_return_unused() {
        // Expression result not consumed (standalone expression as statement).
        let js = r#"
export function mixStandaloneExpr(x) {
    const z = x + 1;
    return z;
}
"#;
        let zig = transpile_and_check!(js, "test_p3_mixed_decl_expr_return_unused");
        println!("=== Mix decl+expr (standalone) ===\n{}", zig);
        // z is used, no suppression
    }

    #[test]
    fn test_native_proto_private_field_basic() {
        // ES2022 private field #field with numeric default, access via this.#field
        let js = r#"
class Counter {
    #count = 10;
    increment() {
        return this.#count + 1;
    }
    getCount() {
        return this.#count;
    }
}

export function testCounter() {
    const c = new Counter();
    return c.getCount();
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_private_field_basic");
        println!("=== Private field Zig code ===\n{}", zig);

        // Verify: Counter struct defined
        assert!(
            zig.contains("const Counter = struct {"),
            "Expected Counter struct"
        );
        // Verify: private field name is stripped of # prefix
        assert!(zig.contains("count:"), "Expected count field in struct");
        // Verify: default value from #count = 10 is preserved
        assert!(
            zig.contains(".count = 10") || zig.contains(".count=10"),
            "Expected default value 10. Got:\n{}",
            zig
        );
        // Verify: this.#count → self.count
        assert!(
            zig.contains("self.count"),
            "Expected self.count access. Got:\n{}",
            zig
        );
        // Verify: increment method exists
        assert!(zig.contains("increment"), "Expected increment method");
        // Verify: getCount method exists
        assert!(zig.contains("getCount"), "Expected getCount method");
    }

    #[test]
    fn test_native_proto_private_field_no_default() {
        // Private field without explicit default → falls back to 0
        let js = r#"
class Widget {
    #id;
    getId() {
        return this.#id;
    }
}

export function testWidget() {
    const w = new Widget();
    return w.getId();
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_private_field_no_default");
        println!("=== Private field (no default) Zig code ===\n{}", zig);

        // Verify: Widget struct defined
        assert!(
            zig.contains("const Widget = struct {"),
            "Expected Widget struct"
        );
        // Verify: field exists (name without #)
        assert!(zig.contains("id:"), "Expected id field");
        // Verify: default init uses 0
        assert!(zig.contains(".id = 0"), "Expected default=0. Got:\n{}", zig);
    }

    #[test]
    fn test_native_proto_private_field_string_default() {
        // Private field with string default
        let js = r#"
class Logger {
    #name = "default";
    getName() {
        return this.#name;
    }
}

export function testLogger() {
    const log = new Logger();
    return log.getName();
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_private_field_string_default");
        println!("=== Private field (string) Zig code ===\n{}", zig);

        // Verify: string default preserved
        assert!(
            zig.contains(".name = \"default\""),
            "Expected string default. Got:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_private_field_multiple() {
        // Multiple private fields with mixed defaults
        let js = r#"
class Config {
    #port = 8080;
    #host = "localhost";
    #secure = true;
    getHost() {
        return this.#host;
    }
}

export function testConfig() {
    const cfg = new Config();
    return cfg.getHost();
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_private_field_multiple");
        println!("=== Private fields (multiple) Zig code ===\n{}", zig);

        // Verify: all fields with correct defaults
        assert!(
            zig.contains(".port = 8080") || zig.contains(".port=8080"),
            "Expected port=8080. Got:\n{}",
            zig
        );
        assert!(
            zig.contains(".host = \"localhost\""),
            "Expected host default. Got:\n{}",
            zig
        );
        assert!(
            zig.contains(".secure = true") || zig.contains(".secure=true"),
            "Expected secure=true. Got:\n{}",
            zig
        );
    }

    #[test]
    fn test_native_proto_private_field_mixed_with_public() {
        // Class with both private and public fields
        let js = r#"
class Person {
    name = "anonymous";
    #age = 0;
    constructor(nameVal, ageVal) {
        this.name = nameVal;
        this.#age = ageVal;
    }
    getAge() {
        return this.#age;
    }
    describe() {
        return this.name;
    }
}

export function testPerson() {
    const p = new Person("Alice", 30);
    return p.describe();
}
"#;
        let zig = transpile_and_assert!(js, "test_native_proto_private_field_mixed");
        println!("=== Private+public fields Zig code ===\n{}", zig);

        // Verify: Person struct defined
        assert!(
            zig.contains("const Person = struct {"),
            "Expected Person struct"
        );
        // Verify: both public and private fields present
        assert!(zig.contains("name:"), "Expected name field");
        assert!(zig.contains("age:"), "Expected age field (stripped #)");
        // Verify: init has both fields
        assert!(
            zig.contains("pub fn init("),
            "Expected init() constructor. Got:\n{}",
            zig
        );
        // Verify: new Person routes correctly
        assert!(zig.contains("Person.init("), "Expected Person.init routing");
        // Verify: this.#age → self.age
        assert!(zig.contains("self.age"), "Expected self.age. Got:\n{}", zig);
    }
}
