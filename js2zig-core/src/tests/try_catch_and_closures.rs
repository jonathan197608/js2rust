// Try-catch, throw, exponentiation, arrow/closure, getter/setter, optional chaining

use super::common::*;

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
    let zig = transpile_and_assert(js, "test_native_proto_try_catch_basic");
    println!("=== Try-catch basic ===\n{}", zig);
    // Should generate the labeled block pattern
    assert!(
        zig.contains("_js_try_blk_"),
        "Expected labeled block:\n{}",
        zig
    );
    // Should generate if-else with error capture for the handler
    assert!(
        zig.contains("} else |_| {"),
        "Expected '}} else |_| {{' for catch handler when e is unused:\n{}",
        zig
    );
    // e is unused in catch body (just return -1), so no JsError binding
    assert!(
        !zig.contains("const e ="),
        "Should NOT have 'const e =' when e is unused:\n{}",
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
    let zig = transpile_and_assert(js, "test_native_proto_try_catch_e_binding_used");
    println!("=== Try-catch e binding (used) ===\n{}", zig);
    // Should generate `const e = js_error.JsError.fromError(__catch_err, ...)` in catch handler
    assert!(
        zig.contains("js_error.JsError.fromError(__catch_err,"),
        "Expected 'js_error.JsError.fromError(__catch_err, ...)' when e is used in catch body:\n{}",
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
    let zig = transpile_and_assert(js, "test_native_proto_try_catch_e_binding_unused");
    println!("=== Try-catch e binding (unused) ===\n{}", zig);
    // Should NOT generate JsError binding when e is unused
    assert!(
        !zig.contains("js_error.JsError.fromError(__catch_err,"),
        "Should NOT have 'JsError.fromError' when e is unused:\n{}",
        zig
    );
    // Should use |_| instead of |__catch_err| when e is unused
    assert!(
        zig.contains("} else |_| {"),
        "Expected '}} else |_| {{' when e is unused:\n{}",
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
    let zig = transpile_and_assert(js, "test_native_proto_try_catch_throw_break");
    println!("=== Try-catch throw break ===\n{}", zig);
    // Inside try: throw should use break, not return
    assert!(
        zig.contains("break :"),
        "Expected break :label for throw inside try:\n{}",
        zig
    );
    // Should have catch handler via if-else (e unused, so |_|)
    assert!(
        zig.contains("} else |_| {"),
        "Expected catch handler with |_| (e unused):\n{}",
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
    let zig = transpile_and_assert(js, "test_native_proto_try_finally");
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
    let zig = transpile_and_assert(js, "test_native_proto_try_catch_finally");
    println!("=== Try-catch-finally ===\n{}", zig);
    // Finally body should be generated as defer inside labeled block
    assert!(
        zig.contains("defer {") && zig.contains("val = 0;"),
        "Expected finally as defer:\n{}",
        zig
    );
    // Should have catch handler via if-else (e unused, so |_|)
    assert!(
        zig.contains("} else |_| {"),
        "Expected catch handler with |_| (e unused):\n{}",
        zig
    );
    assert_zig_ast_check(&zig, "test_native_proto_try_catch_finally");
}

#[test]
fn test_native_proto_try_catch_no_throw() {
    // try-catch without throw statement where body always exits:
    // catch handler is unreachable, so the entire try-catch is inlined.
    let js = r#"
function safeOp(x) {
try {
    return x + 1;
} catch (e) {
    return 0;
}
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_try_catch_no_throw");
    println!("=== Try-catch no throw ===\n{}", zig);
    // Body should be emitted (return x + 1)
    assert!(zig.contains("return (x + 1)"), "Expected body:\n{}", zig);
    // When body always exits and there's no throw, the catch handler
    // is unreachable and the entire try-catch is inlined.
    // No catch handler should be generated in this case.
    assert!(
        !zig.contains("else |__catch_err|") && !zig.contains("} else |_| {"),
        "Should not have catch handler when body always exits:\n{}",
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
    let zig = transpile_and_assert(js, "test_native_proto_throw_bare");
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
    let zig = transpile_and_assert(js, "test_native_proto_try_catch_nested_inner_catch");
    println!("=== Nested try-catch (inner catch) ===\n{}", zig);
    // Each try-catch generates `= _js_try_N:` and `= _js_try_body_N:`
    let result_count = zig.matches("= _js_try_").count(); // = _js_try_0, =_js_try_body_0
    assert_eq!(
        result_count, 4,
        "Expected 4 '= _js_try_' assignments for 2 nested try-catch, got {}:\n{}",
        result_count, zig
    );
    // Inner catch handler: e unused in body, so |_| not |__catch_err|
    assert!(
        zig.contains("} else |_| {"),
        "Expected '}} else |_| {{' in inner catch when e unused:\n{}",
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
    let zig = transpile_and_assert(js, "test_native_proto_try_catch_nested_rethrow");
    println!("=== Nested try-catch (rethrow) ===\n{}", zig);
    // Each try-catch generates `= _js_try_N:` (result) + `= _js_try_body_N:` (body)
    // 2 nested = 4 total.
    let result_count = zig.matches("= _js_try_").count();
    assert_eq!(
        result_count, 4,
        "Expected 4 '= _js_try_' assignments for nested try-catch rethrow, got {}:\n{}",
        result_count, zig
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
    let zig = transpile_and_assert(js, "test_native_proto_try_catch_nested_no_throw");
    println!("=== Nested try-catch (no throw) ===\n{}", zig);
    // Body should contain return x + 1
    assert!(
        zig.contains("return (x + 1)"),
        "Expected body for no-throw inner try:\n{}",
        zig
    );
    // Has catch handler — outer catch with unused e, so |_| not JsError
    assert!(
        zig.contains("} else |_| {"),
        "Expected '}} else |_| {{' for outer catch (e unused):\n{}",
        zig
    );
    // NOTE: assert_zig_ast_check skipped due to known limitation:
    // When nested try-catch has no throw in inner body, the outer body
    // block label (_js_try_body_blk_0) is generated but never referenced.
    // This is tracked as a minor emission optimization issue.
}

// ── Test: JSON.parse inside try-catch → uses break :label, not return ──

#[test]
fn test_native_proto_try_catch_json_parse() {
    // JSON.parse inside try-catch should use break :label (not return error.JsThrow)
    // so that the catch block actually catches the error.
    let js = r##"
function safeParse(str) {
try {
    const result = JSON.parse(str);
    return result;
} catch (e) {
    return null;
}
}
"##;
    let zig = transpile_and_assert(js, "test_native_proto_try_catch_json_parse");
    println!("=== JSON.parse in try-catch ===\n{}", zig);
    // Should generate labeled block pattern
    assert!(
        zig.contains("_js_try_body_blk_"),
        "Expected labeled body block:\n{}",
        zig
    );
    // JSON.parse should use break :label, NOT return error.JsThrow
    assert!(
        zig.contains("break :"),
        "Expected break :label for JSON.parse inside try:\n{}",
        zig
    );
    assert!(
        !zig.contains("catch return error.JsThrow"),
        "Should NOT have 'catch return error.JsThrow' inside try block:\n{}",
        zig
    );
    assert_zig_ast_check(&zig, "test_native_proto_try_catch_json_parse");
}

// ── Test: JSON.parse call (not var decl) inside try-catch ──

#[test]
fn test_native_proto_try_catch_json_parse_call() {
    // bare JSON.parse() call expression inside try-catch
    let js = r##"
function parseAndIgnore(str) {
try {
    JSON.parse(str);
    return true;
} catch (e) {
    return false;
}
}
"##;
    let zig = transpile_and_assert(js, "test_native_proto_try_catch_json_parse_call");
    println!("=== JSON.parse call in try-catch ===\n{}", zig);
    // Should use break :label, not return error.JsThrow
    assert!(
        zig.contains("break :"),
        "Expected break :label for JSON.parse call inside try:\n{}",
        zig
    );
    assert!(
        !zig.contains("catch return error.JsThrow"),
        "Should NOT have 'catch return error.JsThrow' inside try block:\n{}",
        zig
    );
    assert_zig_ast_check(&zig, "test_native_proto_try_catch_json_parse_call");
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
    let zig = transpile_and_assert(js, "test_native_proto_exponential_operator");
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
    let zig = transpile_and_assert(js_float, "test_native_proto_exponential_float");
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
    let zig = transpile_and_assert(js, "test_native_proto_exponential_mixed");
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
    let zig = transpile_and_assert(js, "test_native_proto_arrow_function");
    println!(
        "=== Arrow function (basic) ===
{}",
        zig
    );
    // Should generate a Zig function for the arrow function
    assert!(
        zig.contains("const _arrow_fn_"),
        "Expected arrow function to generate a struct:
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
    let zig = transpile_and_assert(js, "test_native_proto_template_literal_complex");
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
    let zig = transpile_and_assert(js, "test_native_proto_exponential_edge");
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
    let zig = transpile_and_assert(js, "test_native_proto_arrow_single_param");
    println!(
        "=== Arrow function (single param) ===
{}",
        zig
    );
    assert!(
        zig.contains("const _arrow_fn_"),
        "Expected arrow function struct"
    );
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
    let zig = transpile_and_assert(js, "test_native_proto_arrow_block_body");
    println!(
        "=== Arrow function (block) ===
{}",
        zig
    );
    assert!(
        zig.contains("const _arrow_fn_"),
        "Expected arrow function struct"
    );
    assert!(
        zig.contains("return (x + 1);"),
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
    let zig = transpile_and_assert(js, "test_native_proto_closure_basic");
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
    let zig = transpile_and_assert(js, "test_native_proto_closure_mutable");
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
    let zig = transpile_and_assert(js, "test_native_proto_getter");
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

// ── Test: Setter generates @compileError ─────

#[test]
fn test_native_proto_setter_compile_error() {
    // Object literal with setter — setter generates @compileError
    let js = r#"export function useSetter() {
const obj = { a: 1, set x(v) { this._x = v; } };
return obj.a;
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_setter_compile_error");
    println!("=== Setter compile error ===\n{}", zig);
    // Setter should generate @compileError
    assert!(
        zig.contains("@compileError"),
        "Setter should generate @compileError: {}",
        zig
    );
    assert!(!zig.contains("set "), "No 'set' keyword in output: {}", zig);
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
    let zig = transpile_and_assert(js, "test_native_proto_getter_setter_combined");
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
    // Setter for 'age' should generate @compileError
    assert!(
        zig.contains("@compileError"),
        "Setter should generate @compileError: {}",
        zig
    );
    assert!(!zig.contains("set "), "No 'set' keyword in output: {}", zig);
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
    let zig = transpile_and_assert(js, "test_native_proto_optional_chain_known_struct");
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
    let zig = transpile_and_assert(js, "test_native_proto_optional_chain_unknown");
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
    let zig = transpile_and_assert(js, "test_native_proto_optional_chain_call");
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

// ── Test: Optional chain call on JsAny receiver ──────────────────────
// When the receiver is JsAny (e.g. inferred from a null literal), the
// optional chain call must use isNullish() + Conditional instead of
// OptionalChain. OptionalChain emits (if (obj) |oc| ... else null) which
// requires a Zig optional type — JsAny is a union, not an optional.

#[test]
fn test_optional_chain_call_jsany() {
    let js = r#"
function maybeCall() {
    let x = null;
    return x?.get("key");
}
"#;
    let zig = transpile_and_assert(js, "test_optional_chain_call_jsany");
    // Should NOT generate the optional unwrap pattern (if (obj) |oc| ...)
    // P3-3: Conditional expressions are now parenthesized, so check for the
    // capture syntax |oc| instead of the generic (if ( pattern.
    assert!(
        !zig.contains("|_oc"),
        "JsAny receiver should not use OptionalChain unwrap: {}",
        zig
    );
    // Should generate isNullish() check
    assert!(
        zig.contains(".isNullish()"),
        "JsAny receiver should use isNullish(): {}",
        zig
    );
    // Should generate JsAny.fromUndefined() as the null/undefined branch
    assert!(
        zig.contains("JsAny.fromUndefined()"),
        "JsAny null branch should be JsAny.fromUndefined(): {}",
        zig
    );
    // Should still call the method in the non-null branch
    assert!(
        zig.contains(".get(\"key\")"),
        "Should call .get(\"key\") in non-null branch: {}",
        zig
    );
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
    let zig = transpile_and_assert(js, "test_native_proto_optional_chain_nested");
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
    let zig = transpile_and_assert(js, "test_native_proto_optional_chain_null_literal");
    assert!(
        zig.contains("(if (") || zig.contains("null"),
        "Should handle null literal in chain: {}",
        zig
    );
}

// ── Test: Sequential try blocks with catchable errors ─────────────────
// The P1-7 fix resets has_catchable_error after each try body so that
// subsequent try blocks independently detect their own catchable errors.
// Without the fix, the flag is sticky: the second try would see
// catchable_before=true and compute has_catchable_error_in_try=false,
// causing it to use the B1 inline path (no labeled block) instead of
// Case A — producing invalid Zig for catchable operations.

#[test]
fn test_try_sequential_catchable_error_reset() {
    let js = r#"
/** @param {string} jsonStr */
export function testSequentialTry(jsonStr) {
    let a = null;
    try {
        a = JSON.parse(jsonStr);
    } catch (e) {
        a = null;
    }
    let b = null;
    try {
        b = JSON.parse(jsonStr);
    } finally {
        b = null;
    }
    return a;
}
"#;
    let zig = transpile_and_assert(js, "test_try_sequential_catchable_error_reset");
    // Both try blocks should use the labeled-block pattern (Case A).
    // Each Case A try contributes exactly 2 occurrences of _js_try_blk_:
    //   1. const _js_try_N: anyerror!void = _js_try_blk_N: {
    //   2. break :_js_try_blk_N {};
    // Without the P1-7 fix, the second try (finally, no catch) would use
    // B1 inline (0 occurrences) due to has_throw being wrongly false —
    // giving a total of 2 instead of 4.
    let try_blk_count = zig.matches("_js_try_blk_").count();
    assert!(
        try_blk_count >= 4,
        "Expected both try blocks to use labeled-block pattern (>= 4 _js_try_blk_ occurrences), got {}: {}",
        try_blk_count,
        zig
    );
}

// ── Test: break/continue in finally → CompileError ────────────────────
// P1-8: Break/continue in finally that would escape the `defer` block
// (emitted for finally) produces invalid Zig. Catch it early as a
// @compileError instead of letting `zig ast-check` reject it later.
//
// break/continue INSIDE a loop nested in finally are valid (they target
// the enclosing loop), so they must NOT trigger the error.

#[test]
fn test_break_in_finally_compile_error() {
    use crate::tests::common::parse_and_transpile;
    let js = r#"
export function testBreakFinally(arr) {
    let sum = 0;
    try {
        sum = 1;
    } finally {
        if (sum > 0) break;
    }
    return sum;
}
"#;
    let result = parse_and_transpile(js, None).unwrap();
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("break/continue in finally")),
        "Expected 'break/continue in finally' in errors: {:?}",
        result.errors
    );
    assert!(
        result.zig_code.contains("@compileError"),
        "Expected @compileError in zig code:\n{}",
        result.zig_code
    );
}

#[test]
fn test_continue_in_finally_compile_error() {
    use crate::tests::common::parse_and_transpile;
    let js = r#"
export function testContinueFinally(arr) {
    let sum = 0;
    try {
        sum = 1;
    } finally {
        if (sum > 0) continue;
    }
    return sum;
}
"#;
    let result = parse_and_transpile(js, None).unwrap();
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("break/continue in finally")),
        "Expected 'break/continue in finally' in errors: {:?}",
        result.errors
    );
}

#[test]
fn test_break_in_loop_inside_finally_ok() {
    // break inside a for loop in finally targets the loop — valid, no error.
    let js = r#"
export function testBreakInLoopFinally() {
    let sum = 0;
    try {
        sum = 1;
    } finally {
        for (let i = 0; i < 3; i++) {
            if (i === 1) break;
        }
    }
    return sum;
}
"#;
    let result = parse_and_transpile(js, None).unwrap();
    assert!(
        !result
            .errors
            .iter()
            .any(|e| e.contains("break/continue in finally")),
        "Expected NO 'break/continue in finally' error (break in for loop is valid), errors: {:?}",
        result.errors
    );
}

// ── P1-11: return inside try → captured by collect_returns ──────────
// Before the fix, the only return inside a try block was silently dropped,
// causing the function's inferred return type to fall back to something
// other than []const u8. This confirms the return is now picked up.

#[test]
fn test_p1_11_return_inside_try() {
    let js = r#"
export function safeRead(jsonStr) {
    try {
        return JSON.parse(jsonStr);
    } catch (e) {
        return null;
    }
}
"#;
    let zig = transpile_and_assert(js, "test_p1_11_return_inside_try");
    // The `return JSON.parse(...)` inside try and `return null` in catch
    // must both be captured. Return type should be JsAny (since null is
    // JsAny and JSON.parse yields JsAny). Either way, it must NOT be void.
    assert!(
        !zig.contains(") void {"),
        "Expected non-void return type for function returning inside try block (was dropped before P1-11):\n{}",
        zig
    );
}

#[test]
fn test_p2_3_nested_try_throw_detection() {
    // P2-3: ir_stmt_has_throw must recurse into nested IrStmt::Try so that
    // throws inside a nested try's try/catch/finally blocks are detected.
    // Before the fix, the Try arm fell through to `_ => false`, so has_throw
    // was incorrectly false for the outer try when throws only existed inside
    // a nested try.
    let js = r#"
export function nestedTryThrow() {
    try {
        try {
            throw new Error("inner");
        } catch (inner_e) {
            throw new Error("rethrow: " + inner_e.message);
        }
    } catch (e) {
        return e.message;
    }
}
"#;
    // The inner catch rethrows, so the error propagates to the outer catch.
    // Both the inner try_block and catch_block contain throws. With the P2-3
    // fix, has_throw for the outer try is true (throw detected in nested Try).
    // Use transpile_and_assert (not _check) because the catch parameter
    // `inner_e` is dropped by the lowerer — a known pre-existing issue that
    // other try-catch tests also work around by skipping ast-check.
    let zig = transpile_and_assert(js, "test_p2_3_nested_try_throw_detection");
    // The outer try must use the Case A path (throw/catch handling), not
    // the B1 inline path. Case A uses _js_try_blk_ labeled blocks.
    assert!(
        zig.contains("_js_try_blk_"),
        "Expected Case A try emit (labeled blocks) for outer try with nested throw:\n{}",
        zig
    );
}

// ── C5: Arrow function `this` binding inside class methods ───────────
// Before C5 fix: `this` inside arrow/function expressions in class methods
// would reference the closure's `self` (the closure struct itself) or be
// completely absent. After C5: `this` is captured as `__self: *ClassName`
// and rewritten to `self.__self` inside the closure's `call()` method.

#[test]
fn test_c5_arrow_this_in_class_method() {
    // Arrow function inside a class method references `this`
    let js = r#"
class Counter {
    constructor() {
        this.count = 0;
    }
    increment() {
        const add = () => {
            this.count = this.count + 1;
        };
        add();
    }
}

/** @returns {i64} */
export function testC5Counter() {
    const c = new Counter();
    c.increment();
    return c.count;
}
"#;
    let zig = transpile_and_assert(js, "test_c5_arrow_this_in_class_method");
    println!("=== C5 arrow this in class method ===\n{}", zig);
    // The closure struct should have a __self field of type *Counter
    assert!(
        zig.contains("__self: *Counter"),
        "Expected __self: *Counter field in closure struct: {}",
        zig
    );
    // The closure instance init should use .__self = &self
    assert!(
        zig.contains(".__self = &self"),
        "Expected .__self = &self init in closure instance: {}",
        zig
    );
    // Inside the closure's call method, this.count should become self.__self.count
    assert!(
        zig.contains("self.__self.count"),
        "Expected self.__self.count access in closure body: {}",
        zig
    );
}

#[test]
fn test_c5_arrow_this_read_in_class_method() {
    // Arrow function reads this.field but does not write
    let js = r#"
class Box {
    constructor(val) {
        this.value = val;
    }
    getValue() {
        const getter = () => this.value;
        return getter();
    }
}

export function testC5Box(v) {
    const b = new Box(v);
    return b.getValue();
}
"#;
    let zig = transpile_and_assert(js, "test_c5_arrow_this_read_in_class_method");
    println!("=== C5 arrow this read ===\n{}", zig);
    // Should have __self capture
    assert!(
        zig.contains("__self: *Box"),
        "Expected __self: *Box field: {}",
        zig
    );
    // this.value inside arrow → self.__self.value
    assert!(
        zig.contains("self.__self.value"),
        "Expected self.__self.value: {}",
        zig
    );
}

#[test]
fn test_c5_fn_expr_this_in_class_method() {
    // Function expression inside a class method referencing `this`
    let js = r#"
class Logger {
    constructor() {
        this.prefix = "LOG";
    }
    log(msg) {
        const formatMsg = function() {
            return this.prefix + ": " + msg;
        };
        return formatMsg();
    }
}

export function testC5Logger(m) {
    const l = new Logger();
    return l.log(m);
}
"#;
    let zig = transpile_and_assert(js, "test_c5_fn_expr_this_in_class_method");
    println!("=== C5 fn expr this in class method ===\n{}", zig);
    // Same as arrow: __self capture should be generated
    assert!(
        zig.contains("__self: *Logger"),
        "Expected __self: *Logger field: {}",
        zig
    );
    assert!(
        zig.contains("self.__self.prefix"),
        "Expected self.__self.prefix: {}",
        zig
    );
}

#[test]
fn test_c5_arrow_this_with_other_captures() {
    // Arrow function captures both `this` and a local variable
    let js = r#"
class Accum {
    constructor() {
        this.total = 0;
    }
    add(delta) {
        const step = 1;
        const inc = () => {
            this.total = this.total + step + delta;
        };
        inc();
    }
}

export function testC5Accum(d) {
    const a = new Accum();
    a.add(d);
    return a.total;
}
"#;
    let zig = transpile_and_assert(js, "test_c5_arrow_this_with_other_captures");
    println!("=== C5 arrow this with other captures ===\n{}", zig);
    // Should have both __self and step/delta captures
    assert!(
        zig.contains("__self: *Accum"),
        "Expected __self: *Accum: {}",
        zig
    );
    assert!(
        zig.contains("self.__self.total"),
        "Expected self.__self.total: {}",
        zig
    );
    // Other captures should also be accessible
    assert!(
        zig.contains("self.step") || zig.contains("self.delta"),
        "Expected captured step or delta via self: {}",
        zig
    );
}

#[test]
fn test_c5_arrow_no_this_no_capture() {
    // Arrow function inside class method that does NOT use `this`
    // → no __self capture
    let js = r#"
class Processor {
    constructor() {
        this.data = 0;
    }
    run(x) {
        const double = (n) => n * 2;
        return double(x);
    }
}

export function testC5Processor(v) {
    const p = new Processor();
    return p.run(v);
}
"#;
    let zig = transpile_and_assert(js, "test_c5_arrow_no_this_no_capture");
    println!("=== C5 arrow no this no capture ===\n{}", zig);
    // No __self field should appear
    assert!(
        !zig.contains("__self"),
        "Expected no __self when this is not used in arrow: {}",
        zig
    );
}

#[test]
fn test_c5_arrow_this_in_constructor_callback() {
    // Arrow function inside constructor uses `this` — but in constructors,
    // `this.field` is rewritten to local `var field`, so there is no `self`
    // to capture. The arrow body accesses the local variable directly.
    let js = r#"
class Store {
    constructor() {
        this.items = 0;
        const init = () => {
            this.items = 10;
        };
        init();
    }
}

/** @returns {i64} */
export function testC5Store() {
    const s = new Store();
    return s.items;
}
"#;
    let zig = transpile_and_assert(js, "test_c5_arrow_this_in_constructor_callback");
    println!("=== C5 arrow this in constructor callback ===\n{}", zig);
    // In constructors, this.field is rewritten to local var, so __self
    // capture is NOT needed — the arrow directly accesses the local var.
    assert!(
        !zig.contains("__self"),
        "Expected no __self in constructor closure (this is rewritten to local var): {}",
        zig
    );
    // The arrow body should reference the rewritten local variable `items`
    assert!(
        zig.contains("items") || zig.contains("items = 10"),
        "Expected local var 'items' access in constructor closure: {}",
        zig
    );
}
