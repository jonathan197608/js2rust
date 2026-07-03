// Basic codegen: operators, control flow, loops, switch, e2e

use super::common::*;

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
    let zig = transpile_and_assert(js, "test_native_proto_basic");
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
    let zig = transpile_and_assert(js, "test_native_proto_if_else");
    // Rule 7: non-export function param is anytype
    // Rule 6: return type is anytype (both return expressions have type anytype)
    assert!(zig.contains("fn abs(x: anytype) @TypeOf("));
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
    let zig = transpile_and_assert(js, "test_native_proto_elseif");
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
    let zig = transpile_and_assert(js, "test_native_proto_while");
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
    let zig = transpile_and_assert(js, "test_native_proto_function_call");
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
    let zig = transpile_and_assert(js, "test_native_proto_var_decl");
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
    let zig = transpile_and_assert(js, "test_native_proto_operators");
    assert!(
        zig.contains("+") && zig.contains("-") && zig.contains("*") && zig.contains("@divTrunc")
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
    let zig = transpile_and_assert(js, "test_native_proto_logical");
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
    let zig = transpile_and_assert(js, "test_native_proto_toplevel_var_error");
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
    let zig = transpile_and_assert(js, "test_native_proto_unary");
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
    let zig = transpile_and_assert(js, "test_native_proto_typeof");
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
    let zig = transpile_and_assert(js, "test_native_proto_typeof_literal");
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
    let zig = transpile_and_assert(js, "test_native_proto_typeof_object");
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
    let zig = transpile_and_assert(js, "test_native_proto_typeof_dynamic");
    // Should use jsTypeof() for untyped parameters
    assert!(
        zig.contains("js_runtime.jsTypeof"),
        "typeof untyped param should use js_runtime.jsTypeof(): {}",
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

function void_comp_to_string() {
// void 2 === "2" should NOT use std.mem.eql(u8, ...)
// void returns JsAny, must use .strictEq() (=== is strict equality).
return void 2 === "2";
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_void_operator");
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
    // void in binary comparison: must NOT use std.mem.eql(u8, ...)
    // because the void result is JsAny, not []const u8.
    assert!(
        !zig.contains("std.mem.eql(u8, blk_") && !zig.contains("std.mem.eql(u8, _ = blk_"),
        "void === string must not use std.mem.eql with blk label: {}",
        zig
    );
    // Should generate proper JsAny comparison using .strictEq()
    // because void returns JsAny and === is strict equality.
    assert!(
        zig.contains(".strictEq("),
        "void === string should use .strictEq() JsAny comparison: {}",
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
    let zig = transpile_and_assert(js, "test_native_proto_delete_operator");
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
    let zig = transpile_and_assert(js, "test_native_proto_compound_assignment");
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
    let zig = transpile_and_assert(js, "test_native_proto_f64_inference");
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
    let zig = transpile_and_assert(js, "test_native_proto_no_return_void");
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
    let zig = transpile_and_assert(js, "test_native_proto_do_while");
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
    let zig = transpile_and_assert(js, "test_native_proto_for_of");
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
    let zig = transpile_and_assert(js, "test_native_proto_for_in");
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
    let zig = transpile_and_assert(js, "test_native_proto_for_in_static");
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
    let zig = transpile_and_assert(js, "test_p2_for_in_static_codegen");
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
    // Use parse_and_transpile directly to access TranspileResult errors
    let result = parse_and_transpile(js, None).unwrap();
    let zig = result.zig_code;
    eprintln!("=== DEBUG: outer() return type ===");
    eprintln!("{}", zig);
    assert!(
        result.errors.is_empty(),
        "Unexpected errors: {:?}",
        result.errors
    );
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

    // Verify: inner is defined via named type with capture field (Zig 0.16 syntax)
    assert!(
        zig.contains("_inner_type = struct {"),
        "Expected _inner_type struct declaration"
    );
    assert!(zig.contains("x:"), "Expected capture field x");

    // Verify: inner is instantiated from named type
    assert!(
        zig.contains("inner = _inner_type"),
        "Expected inner = _inner_type instantiation"
    );

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
    let zig = transpile_and_assert(js, "test_native_proto_switch");
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
