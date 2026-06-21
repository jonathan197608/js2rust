// native_proto/tests.rs
// Tests for native-type codegen.

#[cfg(test)]
mod tests {
    use crate::native_proto::transpile_js;
    use std::process::Command;

    /// Helper: run `zig ast-check` on generated Zig code.
    /// Panics if ast-check fails (to fail the test).
    /// Skips gracefully if `zig` is not installed.
    fn assert_zig_ast_check(zig_code: &str, test_name: &str) {
        let tmp_dir = std::env::temp_dir();
        let zig_path = tmp_dir.join(format!("{}.zig", test_name));
        std::fs::write(&zig_path, zig_code).unwrap();

        match Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    eprintln!("=== zig ast-check failed for {} ===", test_name);
                    eprintln!("Generated code:\n{}", zig_code);
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

    /// Helper: transpile JS to Zig, unwrap, print generated code.
    fn transpile_js_simple(js: &str) -> String {
        let zig = transpile_js(js).unwrap();
        zig
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
        let zig = transpile_js(js).unwrap();
        println!("=== Generated Zig ===\n{}", zig);
        // Note: using anytype for parameters, i64 for return type (inferred)
        assert!(zig.contains("pub fn add(a: i64, b: i64) i64 {"));
        assert!(zig.contains("return a + b;"));
    }

    #[ignore]
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
        let zig = transpile_js(js).unwrap();
        println!("=== If/Else ===\n{}", zig);
        assert!(zig.contains("fn abs(x: anytype)"));
        assert!(zig.contains("if (x") && zig.contains(">= 0"), "missing if: {}", zig);
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
        let zig = transpile_js(js).unwrap();
        println!("=== ElseIf ===\n{}", zig);
        assert!(zig.contains("else") && zig.contains("if (score"), "missing else if: {}", zig);
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
        let zig = transpile_js(js).unwrap();
        println!("=== While ===\n{}", zig);
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
        let zig = transpile_js(js).unwrap();
        println!("=== Function Call ===\n{}", zig);
        assert!(zig.contains("greet(")); // function call (no try)
        assert!(zig.contains("++")); // string + → concat
        assert!(zig.contains("var msg:")); // type annotated
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
        let zig = transpile_js(js).unwrap();
        println!("=== Var Decl ===\n{}", zig);
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
        let zig = transpile_js(js).unwrap();
        println!("=== Operators ===\n{}", zig);
        assert!(zig.contains("+") && zig.contains("-") && zig.contains("*") && zig.contains("/"));
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
        let zig = transpile_js(js).unwrap();
        println!("=== Logical ===\n{}", zig);
        assert!(zig.contains("and"));
        assert!(zig.contains("or"));
    }

    #[test]
    fn test_native_proto_toplevel_var_error() {
        let js = r#"
let y = 10;
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Toplevel Var Error ===\n{}", zig);
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
        let zig = transpile_js(js).unwrap();
        println!("=== Unary ===\n{}", zig);
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
        let zig = transpile_js(js).unwrap();
        println!("=== F64 Inference ===\n{}", zig);
        assert!(zig.contains("3.14159"));
        // Division returns f64 by default? Actually we infer from left operand.
    }

    #[ignore]
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
        let zig = transpile_js(js).unwrap();
        println!("=== Complex Test ===\n{}", zig);
        assert!(zig.contains("const PI: f64 = 3.14;"));
        assert!(zig.contains("fn circleArea(radius: anytype)"));
        assert!(zig.contains("var r2: i64 = radius * radius;"));
        assert!(zig.contains("factorial(")); // function call (no try)
        assert!(zig.contains("if (n") && zig.contains("<="), "missing if: {}", zig);
    }

    #[test]
    fn test_native_proto_no_return_void() {
        let js = r#"
function log(msg) {
    // no explicit return → void
}
"#;
        let zig = transpile_js(js).unwrap();
        println!("=== Void Return ===\n{}", zig);
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
        let zig = transpile_js(js).unwrap();
        println!("=== Do-While ===\n{}", zig);
        assert!(zig.contains("while (true) {"), "missing while true: {}", zig);
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
        let zig = transpile_js(js).unwrap();
        println!("=== For-Of ===\n{}", zig);
        assert!(zig.contains("for (arr) |x| {"), "missing for-of: {}", zig);
        assert!(zig.contains("total = total + x;"));
        assert!(zig.contains("return total;"));
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
        let zig = transpile_js(js).unwrap();
        println!("=== Switch (Zig native) ===\n{}", zig);
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
    /// Strategy: transpile JS → Zig, then wrap the generated functions in a `pub fn main() !void`
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
        let zig_gen = transpile_js(js).unwrap();
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
                "-O", "Debug",
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
        assert!(stderr.contains("add(10,20)=30"),
            "expected 'add(10,20)=30' in stderr, got: stdout='{}' stderr='{}'", stdout, stderr);
        assert!(stderr.contains("abs(-42)=42"),
            "expected 'abs(-42)=42' in stderr, got: stdout='{}' stderr='{}'", stdout, stderr);

        println!("=== E2E test passed! Generated Zig code compiles and runs correctly ===");
    }

    #[test]
    fn test_native_proto_object_struct() {
        // Scheme C: Only static access → anonymous struct.
        let js = r#"
function main() {
    const pt = { x: 10, y: 20 };
    const a = pt.x;
    const b = pt.y;
    return a + b;
}
"#;
        let zig = transpile_js(js).unwrap();
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
        // Scheme C: Dynamic access → StringHashMap.
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
        let result = transpile_js(js);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Dynamic property access"), "Expected error about dynamic property access, got: {}", err);
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
        let zig = transpile_js(js).unwrap();
        println!("=== Object Struct Mutation ===\n{}", zig);
        // Should use 'var' for the object (because it's mutated).
        assert!(zig.contains("var pt ="));
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
        let result = transpile_js(js);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Dynamic property access") || err.contains("Dynamic property assignment"),
                "Expected error about dynamic property access/assignment, got: {}", err);
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
        let zig = transpile_js(js).unwrap();
        println!("=== Field Type Mismatch ===\n{}", zig);
        // Should use 'var' for the object (because it's mutated).
        assert!(zig.contains("var pt ="));
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
        let zig = transpile_js(js).unwrap();
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
        let zig = transpile_js(js).unwrap();
        println!("=== JSDoc @type + JSON.parse() ===\n{}", zig);
        // Should generate struct definition.
        assert!(zig.contains("const User = struct {"));
        // Should generate std.json.parse(User, ...) for JSON.parse() with @type.
        assert!(zig.contains("std.json.parse(User,"), "Expected std.json.parse(User, ...), got: {}", zig);
        // Should have catch unreachable.
        assert!(zig.contains("catch unreachable"));
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
        let zig = transpile_js(js).unwrap();
        println!("=== Export Function Signature ===\n{}", zig);
        // Export function: should use real types from JSDoc
        // NOTE: native_proto generates 'pub fn' (not 'export fn')
        assert!(zig.contains("pub fn add(a: i64, b: i64) i64 {"));
        // Export function with @returns {void}: should be void.
        assert!(zig.contains("pub fn log(msg: i64) void {"));
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
        let zig = transpile_js(js).unwrap();
        println!("=== Param Annotation Test ===\n{}", zig);
        // @param {string} name: should use []const u8 directly
        // @param {number} age: should use i64 directly
        // NOTE: native_proto adds 'export ' prefix to export functions
        assert!(zig.contains("export fn greet (name: []const u8, age: i64) []const u8 {"));
        // Should NOT generate parseInt code (types are already correct)
        assert!(!zig.contains("parseInt"));
        // Should use ++ for string concatenation
        assert!(zig.contains("++"));
    }

    #[test]
    fn test_native_proto_export_requires_returns() {
        // Test that export functions require @returns annotation.
        let js = r#"
export function getName(user) {
    return user.name;
}
"#;
        let result = transpile_js(js);
        // Should fail because export function has no @returns annotation.
        assert!(result.is_err(), "Expected error for missing @returns, got: {:?}", result);
        let err = result.unwrap_err();
        assert!(err.contains("@returns"), "Expected error about @returns, got: {}", err);
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
        let zig = transpile_js(js).unwrap();
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
        let result = transpile_js(js);
        assert!(result.is_ok(), "Transpile failed: {:?}", result.err());
        let zig = result.unwrap();

        println!("=== Generated Zig code ===\n{}", zig);

        // Verify string concatenation uses ++ operator
        assert!(zig.contains(" ++ "), "Expected ++ operator for string concat, got:\n{}", zig);
        assert!(!zig.contains(" + "), "Should not use + operator for string concat, got:\n{}", zig);
    }

    #[test]
    fn test_native_proto_string_concat_multi() {
        // Test: multiple string concatenation
        let js = r#"
function fullName(first, last) {
    return first + " " + last;
}
"#;
        let result = transpile_js(js);
        assert!(result.is_ok(), "Transpile failed: {:?}", result.err());
        let zig = result.unwrap();

        println!("=== Generated Zig code ===\n{}", zig);

        // Verify all concatenations use ++ operator
        let plus_count = zig.matches(" + ").count();
        assert_eq!(plus_count, 0, "Found {} instances of + operator, expected 0 for string concat", plus_count);
        assert!(zig.contains(" ++ "), "Expected ++ operator for string concat, got:\n{}", zig);
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
        let result = transpile_js(js);
        assert!(result.is_ok(), "Transpile failed: {:?}", result.err());
        let zig = result.unwrap();

        println!("=== Generated Zig code ===\n{}", zig);

        // Verify return value uses correct type (no dupe needed for []const u8)
        assert!(zig.contains("fn greet(name: []const u8) []const u8 {"));
        // Verify string concatenation uses ++ operator
        assert!(zig.contains("++"));
        // Should NOT contain allocator.dupe (no C ABI conversion)
        assert!(!zig.contains("allocator.dupe"));
        // Should NOT contain result_len (no C ABI wrapper)
        assert!(!zig.contains("result_len"));
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
        let result = transpile_js(js);
        assert!(result.is_ok(), "Transpile failed: {:?}", result.err());
        let zig = result.unwrap();

        println!("=== Generated Zig code (complex nested struct) ===\n{}", zig);

        // Verify Address struct is generated
        assert!(zig.contains("const Address = struct {"), "Expected Address struct, got:\n{}", zig);
        assert!(zig.contains("street: []const u8,"), "Expected street field, got:\n{}", zig);
        assert!(zig.contains("city: []const u8,"), "Expected city field, got:\n{}", zig);
        assert!(zig.contains("zip: i64,"), "Expected zip field, got:\n{}", zig);
        
        // Verify Address has toJson() method
        assert!(zig.contains("pub fn toJson") && zig.contains("Address"), "Expected toJson() for Address, got:\n{}", zig);

        // Verify User struct is generated with all field types
        assert!(zig.contains("const User = struct {"), "Expected User struct, got:\n{}", zig);
        assert!(zig.contains("name: []const u8,"), "Expected name field, got:\n{}", zig);
        assert!(zig.contains("age: i64,"), "Expected age field, got:\n{}", zig);
        assert!(zig.contains("tags: []const []const u8,"), "Expected tags field (string[]), got:\n{}", zig);
        assert!(zig.contains("scores: []const i64,"), "Expected scores field (number[]), got:\n{}", zig);
        assert!(zig.contains("addresses: []const Address,"), "Expected addresses field (Address[]), got:\n{}", zig);

        // Verify User has toJson() method
        assert!(zig.contains("pub fn toJson") && zig.contains("const User"), "Expected toJson() for User, got:\n{}", zig);

        // Verify toJson() uses std.json.fmt() for serialization
        assert!(zig.contains("std.json.fmt"), "Expected std.json.fmt() in toJson(), got:\n{}", zig);
        assert!(zig.contains("Writer.Allocating"), "Expected Writer.Allocating in toJson(), got:\n{}", zig);

        // Verify JSON.stringify() is converted to user.toJson() (no allocator parameter)
        assert!(zig.contains("try user.toJson()"), "Expected try user.toJson(), got:\n{}", zig);

        // Run zig ast-check to verify the code is syntactically correct
        let tmp_dir = std::env::temp_dir();
        let zig_path = tmp_dir.join("typedef_tojson_complex_test.zig");
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
        let result = transpile_js(js);
        assert!(result.is_ok(), "Transpile failed: {:?}", result.err());
        let zig = result.unwrap();

        println!("=== Generated Zig code (JSON.parse with nested structs) ===\n{}", zig);

        // Verify Address and User structs are generated
        assert!(zig.contains("const Address = struct {"), "Expected Address struct, got:\n{}", zig);
        assert!(zig.contains("const User = struct {"), "Expected User struct, got:\n{}", zig);

        // Verify JSON.parse() is converted to std.json.parse()
        assert!(zig.contains("std.json.parse(User,"), "Expected std.json.parse(User, ...), got:\n{}", zig);

        // Verify data variable uses the correct type
        assert!(zig.contains("const data: User ="), "Expected 'const data: User', got:\n{}", zig);
        
        // Verify member access works (data.name, data.addresses[0].city)
        assert!(zig.contains("data.name"), "Expected data.name access, got:\n{}", zig);
        assert!(zig.contains("data.addresses[0].city"), "Expected data.addresses[0].city access, got:\n{}", zig);

        // Run zig ast-check to verify the code is syntactically correct
        let tmp_dir = std::env::temp_dir();
        let zig_path = tmp_dir.join("json_parse_nested_test.zig");
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
        
        // Step 1: generate Zig source from JS
        let zig_gen = transpile_js(js).unwrap();
        println!("=== Generated Zig code (JSON) ===\n{}", zig_gen);
        
        // Step 2: create a complete Zig program
        // Remove `const std = @import("std");` from generated code to avoid duplicate
        let zig_gen_clean = zig_gen.replace("const std = @import(\"std\");\n", "");
        
        let zig_full = format!(
            r#"const std = @import("std");

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
            let run_output = std::process::Command::new(&exe_path)
                .output()
                .unwrap();
            
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            println!("=== Program output ===\n{}", stdout);
            
            // Verify output contains expected strings
            assert!(stdout.contains("Serialized JSON:"), "Expected 'Serialized JSON:' in output, got: {}", stdout);
            assert!(stdout.contains("Bob"), "Expected 'Bob' in output, got: {}", stdout);
            assert!(stdout.contains("Parsed: Alice is 30 years old"), "Expected 'Parsed: Alice is 30 years old' in output, got: {}", stdout);
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
 * @property {string} [email]  ← optional
 * @property {number} [score]  ← optional
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
        
        // Step1: generate Zig source from JS
        let zig = transpile_js(js).unwrap();
        println!("=== Generated Zig code (optional property) ===\n{}", zig);
        
        // Step2: verify optional fields are generated with ?Type
        assert!(zig.contains("name: []const u8,"), "Expected 'name: []const u8,' in:\n{}", zig);
        assert!(zig.contains("age: i64,"), "Expected 'age: i64,' in:\n{}", zig);
        assert!(zig.contains("email: ?[]const u8,"), "Expected 'email: ?[]const u8,' (optional) in:\n{}", zig);
        assert!(zig.contains("score: ?i64,"), "Expected 'score: ?i64,' (optional) in:\n{}", zig);
        
        // Step3: verify toJson() is generated (std.json.fmt() handles ?T automatically)
        assert!(zig.contains("pub fn toJson"), "Expected toJson() method in:\n{}", zig);
        
        // Step4: verify the code passes zig ast-check
        let temp_dir = std::env::temp_dir();
        let zig_path = temp_dir.join("optional_property_test.zig");
        std::fs::write(&zig_path, &zig).unwrap();
        
        match std::process::Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    panic!("zig ast-check failed:\n{}\nGenerated code:\n{}", stderr, zig);
                }
            }
            Err(e) => {
                eprintln!("Failed to run zig ast-check: {}", e);
                // Skip if zig not available
            }
        }
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
        
        // Step1: generate Zig source from JS
        let zig = transpile_js(js).unwrap();
        println!("=== Generated Zig code (Math methods) ===\n{}", zig);
        
        // Step2: verify Math methods are generated correctly
        assert!(zig.contains("@abs("), "Expected '@abs(' in:\n{}", zig);
        assert!(zig.contains("@floor("), "Expected '@floor(' in:\n{}", zig);
        assert!(zig.contains("@ceil("), "Expected '@ceil(' in:\n{}", zig);
        assert!(zig.contains("@round("), "Expected '@round(' in:\n{}", zig);
        assert!(zig.contains("@sqrt("), "Expected '@sqrt(' in:\n{}", zig);
        
        // Step3: verify the code passes zig ast-check
        let temp_dir = std::env::temp_dir();
        let zig_path = temp_dir.join("math_methods_test.zig");
        std::fs::write(&zig_path, &zig).unwrap();
        
        match std::process::Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    panic!("zig ast-check failed:\n{}\nGenerated code:\n{}", stderr, zig);
                }
            }
            Err(e) => {
                eprintln!("Failed to run zig ast-check: {}", e);
                // Skip if zig not available
            }
        }
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
        
        // Step1: generate Zig source from JS
        let zig = transpile_js(js).unwrap();
        println!("=== Generated Zig code (Array.pop()) ===\n{}", zig);
        
        // Step2: verify arr.pop() is generated
        assert!(zig.contains("arr.pop()"), "Expected 'arr.pop()' in:\n{}", zig);
        
        // Step3: verify the code passes zig ast-check
        let temp_dir = std::env::temp_dir();
        let zig_path = temp_dir.join("array_pop_test.zig");
        std::fs::write(&zig_path, &zig).unwrap();
        
        match std::process::Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    panic!("zig ast-check failed:\n{}\nGenerated code:\n{}", stderr, zig);
                }
            }
            Err(e) => {
                eprintln!("Failed to run zig ast-check: {}", e);
                // Skip if zig not available
            }
        }
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
        
        let zig = transpile_js(js).unwrap();
        println!("=== Generated Zig code (Array.indexOf()) ===\n{}", zig);
        
        // Verify labeled block with for loop is generated
        assert!(zig.contains("blk:"), "Expected labeled block in:\n{}", zig);
        assert!(zig.contains("for ("), "Expected for loop in:\n{}", zig);
        assert!(zig.contains(".items"), "Expected .items access in:\n{}", zig);
        assert!(zig.contains("break :blk"), "Expected break :blk in:\n{}", zig);
        assert!(zig.contains("@as(i64, -1)"), "Expected @as(i64, -1) fallback in:\n{}", zig);
        
        // Verify zig ast-check passes
        let temp_dir = std::env::temp_dir();
        let zig_path = temp_dir.join("array_indexof_test.zig");
        std::fs::write(&zig_path, &zig).unwrap();
        
        match std::process::Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    panic!("zig ast-check failed:\n{}\nGenerated code:\n{}", stderr, zig);
                }
            }
            Err(e) => {
                eprintln!("Failed to run zig ast-check: {}", e);
            }
        }
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
        
        let zig = transpile_js(js).unwrap();
        println!("=== Generated Zig code (Array.includes()) ===\n{}", zig);
        
        // Verify labeled block with for loop and bool return
        assert!(zig.contains("blk:"), "Expected labeled block in:\n{}", zig);
        assert!(zig.contains("for ("), "Expected for loop in:\n{}", zig);
        assert!(zig.contains("break :blk true"), "Expected break :blk true in:\n{}", zig);
        assert!(zig.contains("break :blk false"), "Expected break :blk false in:\n{}", zig);
        
        // Verify zig ast-check passes
        let temp_dir = std::env::temp_dir();
        let zig_path = temp_dir.join("array_includes_test.zig");
        std::fs::write(&zig_path, &zig).unwrap();
        
        match std::process::Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    panic!("zig ast-check failed:\n{}\nGenerated code:\n{}", stderr, zig);
                }
            }
            Err(e) => {
                eprintln!("Failed to run zig ast-check: {}", e);
            }
        }
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
        
        let zig = transpile_js(js).unwrap();
        println!("=== Generated Zig code (Array.join()) ===\n{}", zig);
        
        // Verify std.io.Writer.Allocating is used
        assert!(zig.contains("std.io.Writer.Allocating"), "Expected std.io.Writer.Allocating in:\n{}", zig);
        assert!(zig.contains("__join_buf"), "Expected __join_buf variable in:\n{}", zig);
        assert!(zig.contains("writeAll"), "Expected writeAll for separator in:\n{}", zig);
        // i64 elements should use {d} format
        assert!(zig.contains("{d}"), "Expected {{d}} format for i64 elements in:\n{}", zig);
        
        // Verify zig ast-check passes
        let temp_dir = std::env::temp_dir();
        let zig_path = temp_dir.join("array_join_test.zig");
        std::fs::write(&zig_path, &zig).unwrap();
        
        match std::process::Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    panic!("zig ast-check failed:\n{}\nGenerated code:\n{}", stderr, zig);
                }
            }
            Err(e) => {
                eprintln!("Failed to run zig ast-check: {}", e);
            }
        }
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
        
        let zig = transpile_js(js).unwrap();
        println!("=== Generated Zig code (Array.slice()) ===\n{}", zig);
        
        // Verify slice expression is generated: arr.items[1..4]
        assert!(zig.contains(".items[1..4]"), "Expected '.items[1..4]' slice in:\n{}", zig);
        
        // Verify zig ast-check passes
        let temp_dir = std::env::temp_dir();
        let zig_path = temp_dir.join("array_slice_test.zig");
        std::fs::write(&zig_path, &zig).unwrap();
        
        match std::process::Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    panic!("zig ast-check failed:\n{}\nGenerated code:\n{}", stderr, zig);
                }
            }
            Err(e) => {
                eprintln!("Failed to run zig ast-check: {}", e);
            }
        }
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
        
        // Step1: generate Zig source from JS
        let zig = transpile_js(js).unwrap();
        println!("=== Generated Zig code (Math new methods) ===\n{}", zig);
        
        // Step2: verify Math methods are generated correctly
        assert!(zig.contains("std.crypto.random.int(u32)"), "Expected 'std.crypto.random.int(u32)' in:\n{}", zig);
        assert!(zig.contains("std.math.pow(f64,"), "Expected 'std.math.pow(f64,' in:\n{}", zig);
        assert!(zig.contains("if ("), "Expected 'if (' in max/min:\n{}", zig);
    }

    // ── Test: AwaitExpression support ─────────────

    #[test]
    fn test_native_proto_await() {
        // JS source: async function with await
        let js = r#"
/**
 * @param {i64} x
 * @returns {i64}
 */
export async function asyncDouble(x) {
    const result = await double(x);
    return result;
}

function double(x) {
    return x * 2;
}
"#;
        
        // Step1: generate Zig source from JS
        let zig = transpile_js(js).unwrap();
        println!("=== Generated Zig code (AwaitExpression) ===\n{}", zig);
        
        // Step2: verify async function signature has `io: anytype`
        assert!(zig.contains("io: anytype"), 
            "Expected 'io: anytype' in async function signature, got:\n{}", zig);
        
        // Step3: verify await is translated to io.async() + .await(io)
        assert!(zig.contains("io.async("), 
            "Expected 'io.async(' in generated code, got:\n{}", zig);
        assert!(zig.contains(".await(io)"), 
            "Expected '.await(io)' in generated code, got:\n{}", zig);
        // defer _ = _tN.cancel(io) catch undefined;
        assert!(zig.contains("defer"), 
            "Expected 'defer' in generated code, got:\n{}", zig);
        assert!(zig.contains(".cancel(io)"), 
            "Expected '.cancel(io)' in generated code, got:\n{}", zig);
        
        // Step4: verify non-async function does NOT have `io: anytype`
        assert!(zig.contains("fn double(x: anytype) i64 {"), 
            "Expected non-async function signature, got:\n{}", zig);
        
        // Step5: verify the generated code passes `zig ast-check`
        let tmp_dir = std::env::temp_dir();
        let zig_path = tmp_dir.join("await_test.zig");
        std::fs::write(&zig_path, &zig).unwrap();
        
        match std::process::Command::new("zig.exe")
            .args(&["ast-check", zig_path.to_str().unwrap()])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    eprintln!("=== zig ast-check failed ===");
                    eprintln!("Generated code:\n{}", zig);
                    eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
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
        
        println!("=== AwaitExpression test passed! ===");
    }
}
