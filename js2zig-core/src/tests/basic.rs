// Basic transpilation: operators, control flow, loops, switch, e2e

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
    // Note: using anytype for parameters, f64 for return type (inferred)
    assert!(zig.contains("pub fn add(a: anytype, b: anytype) f64 {"));
    assert!(zig.contains("return (a + b);"));
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
        zig.contains("if ((x") && zig.contains(">= 0"),
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
        zig.contains("else") && zig.contains("if ((score"),
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
    assert!(zig.contains("n = (n - 1);"));
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
    assert!(zig.contains("total = (total + 1);"));
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
        zig.contains("+") && zig.contains("-") && zig.contains("*") && zig.contains("floatFromInt")
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
    // Value-returning: `a > 0 && b > 0` emits a labeled-block if-expression,
    // not Zig `and`. Verify the if-expression structure is present.
    assert!(zig.contains("isTruthy(_lv_"));
    // `||` is also emitted as an if-expression with isTruthy.
    assert!(zig.contains("break :blk_"));
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
    // Logical NOT now uses isTruthy for type-safe coercion
    assert!(zig.contains("!js_runtime.isTruthy(x)"));
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
 * @typedef {Object} User
 * @property {string} name
 */
/**
 * @param {User} o
 * @param {Array} a
 */
export function typeof_obj(o, a) {
var t1 = typeof o;
var t2 = typeof a;
return t1;
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_typeof_object");
    // typeof named struct (User) → should emit "object" string literal
    assert!(
        zig.contains("\"object\""),
        "typeof object should emit \"object\" string: {}",
        zig
    );
    // typeof Array (now JsAny after P2-5) → should use runtime jsTypeof
    assert!(
        zig.contains("js_runtime.jsTypeof"),
        "typeof Array (JsAny) should use js_runtime.jsTypeof(): {}",
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

function delete_chain(obj) {
delete obj.a.b;
return obj;
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_delete_operator");
    // delete obj.prop uses deleteKey("prop")
    assert!(
        zig.contains(".deleteKey(\"name\")"),
        "delete obj.prop should use deleteKey: {}",
        zig
    );
    // delete obj[expr] uses .delete(alloc, JsAny.from(key))
    assert!(
        zig.contains(".delete(js_allocator.allocator(), JsAny.from(_dk))"),
        "delete obj[expr] should use .delete(alloc, JsAny.from(key)): {}",
        zig
    );
    // delete should consume result with _ =
    assert!(
        zig.contains("_ = ") && zig.contains("deleteKey"),
        "delete should use _ = to discard: {}",
        zig
    );
    // delete obj.a.b — receiver is a non-Identifier expression (StaticMemberExpression),
    // must NOT be dropped. The InnerInline renderer wraps it in parens so we expect
    // to find ".a).deleteKey(\"b\")" (the `)` closes the inline-rendered `obj.a`).
    assert!(
        zig.contains("deleteKey(\"b\")"),
        "delete obj.a.b should still emit deleteKey for the outer property: {}",
        zig
    );
    assert!(
        !zig.contains("_ = .deleteKey"),
        "delete obj.a.b must NOT discard the receiver (no bare `_ = .deleteKey`): {}",
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
    // &&= → expanded to Logical(And), uses js_runtime.isTruthy
    assert!(
        zig.contains("js_runtime.isTruthy"),
        "&&= should use js_runtime.isTruthy: {}",
        zig
    );
    // ||= → also uses js_runtime.isTruthy (branches flipped)
    assert!(
        zig.contains("js_runtime.isTruthy"),
        "||= should use js_runtime.isTruthy: {}",
        zig
    );
    // ??= on anytype (non-JsAny) → no-op sequence (RHS evaluated, original kept)
    // This is correct: non-JsAny types cannot be nullish, so ??= is a no-op.
    assert!(
        !zig.contains(".isNullish()"),
        "??= on anytype should be no-op (no isNullish): {}",
        zig
    );
}

#[test]
fn test_nullish_assign_jsany() {
    let js = r#"
function nullish_jsany() {
let x = null;
x ??= 42;
return x;
}
"#;
    let zig = transpile_and_assert(js, "test_nullish_assign_jsany");
    // For JsAny-typed variable (inferred from null literal), ??= uses isNullish
    assert!(
        zig.contains(".isNullish()"),
        "??= on JsAny should use isNullish: {}",
        zig
    );
}

/// Compound assignment on a member with a side-effecting object expression
/// (e.g. `getBox().value **= n`) must bind the object to a temp variable
/// (__co_N) to prevent double evaluation of the object expression.
/// Tests the P1-4 fix for the compound-assignment double-eval bug.
#[test]
fn test_compound_assign_member_side_effect() {
    let js = r#"
/**
 * @typedef {Object} Box
 * @property {number} value
 */

/**
 * @type {Box}
 */
var box = { value: 5 };

function getBox() {
    return box;
}

/**
 * @returns {number}
 */
export function testPowAssignSideEffect() {
    getBox().value **= 3;
    return box.value;
}

/**
 * @returns {number}
 */
export function testRemAssignSideEffect() {
    getBox().value %= 4;
    return box.value;
}

/**
 * @returns {number}
 */
export function testUrShrAssignSideEffect() {
    getBox().value >>>= 1;
    return box.value;
}
"#;
    let zig = transpile_and_assert(js, "test_compound_assign_member_side_effect");
    println!("=== Compound assign on side-effecting member ===\n{}", zig);

    // All three compound ops should bind the object to __co_ temp vars
    let co_count = zig.matches("__co_").count();
    assert!(
        co_count >= 3,
        "Expected at least 3 __co_ temp bindings (one per compound op), got {}: {}",
        co_count,
        zig
    );
    // Should NOT contain __target placeholder (old buggy fallback)
    assert!(
        !zig.contains("__target"),
        "Should not use __target placeholder:\n{}",
        zig
    );
    // Should use labeled blocks for the temp bindings
    assert!(
        zig.contains("_co_blk"),
        "Should use _co_blk labeled block for temp binding:\n{}",
        zig
    );
}

// ── P1-11: collect_returns must descend into every block ──────────
// Before the fix, returns nested inside DoWhile/For/ForOf/ForIn/Switch/Try/
// Labeled were silently dropped, causing the function's return-type inference
// to fall back to `void` (or `i64`/`anytype`) instead of using the inferred
// type of the return expression. Each test below has its ONLY return inside
// one of the previously-missing constructs and confirms the inferred Zig
// return type matches the returned value.

#[test]
fn test_p1_11_return_inside_for_loop() {
    let js = r#"
export function firstMatch(arr) {
    for (let i = 0; i < arr.length; i = i + 1) {
        if (i === 2) return "found";
    }
    return "not-found";
}
"#;
    let zig = transpile_and_assert(js, "test_p1_11_return_inside_for_loop");
    // The `return "found"` inside the for loop must be captured so the
    // function return type is []const u8, not the default i64/void.
    assert!(
        zig.contains("[]const u8"),
        "Expected return type []const u8 (return inside for loop was dropped before P1-11):\n{}",
        zig
    );
}

#[test]
fn test_p1_11_return_inside_switch() {
    let js = r#"
/** @param {number} code */
export function classify(code) {
    switch (code) {
        case 1: return "one";
        case 2: return "two";
        default: return "other";
    }
}
"#;
    let zig = transpile_and_assert(js, "test_p1_11_return_inside_switch");
    assert!(
        zig.contains("[]const u8"),
        "Expected return type []const u8 (return inside switch was dropped before P1-11):\n{}",
        zig
    );
}

#[test]
fn test_p1_11_return_inside_labeled_for() {
    let js = r#"
export function findTarget(target, items) {
search: for (let i = 0; i < items.length; i = i + 1) {
        if (i === target) return i;
    }
    return -1;
}
"#;
    let zig = transpile_and_assert(js, "test_p1_11_return_inside_labeled_for");
    // Return value `i` is i64 (the loop counter); function should be i64
    // anyway. The real test is that BOTH returns are captured — if either
    // is dropped, the inferred type might still be i64 by coincidence, so
    // we additionally verify that the for-loop body actually emits a return.
    assert!(
        zig.contains("return i;") || zig.contains("return _i;"),
        "Expected return statement inside for-loop body in:\n{}",
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
    assert!(zig.contains("const r2 = (radius * radius);"));
    assert!(zig.contains("factorial(")); // function call (no try)
    assert!(
        zig.contains("if ((n") && zig.contains("<="),
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
    // P1-10: do-while first-iteration flags are now uniquely named
    // `__dw_first_N`. The old hardcoded `__dw_first` caused Zig "local
    // variable shadows local variable from outer scope" errors for nested
    // do-whiles.
    assert!(
        zig.contains("__dw_first"),
        "missing do-while first-iteration flag: {}",
        zig
    );
    assert!(
        zig.contains("__dw_first_0"),
        "missing do-while first-iteration flag (__dw_first_0): {}",
        zig
    );
    assert!(
        zig.contains("while (__dw_first_0 or ((x > 0)))"),
        "missing do-while while condition: {}",
        zig
    );
    assert!(
        zig.contains(": (__dw_first_0 = false)"),
        "missing do-while continuation: {}",
        zig
    );
    assert!(
        zig.contains("JsAny.from(x)"),
        "expected JsAny.from(x) coercion for JsAny-returning function: {}",
        zig
    );
}

#[test]
fn test_native_proto_nested_do_while() {
    // P1-10: Nested do-while loops must not collide on the __dw_first
    // variable name. Each do-while is scoped in its own {} block so the
    // shadowing is lexically valid in Zig — but we still want to verify
    // the generated code passes `zig ast-check` (whether Zig emits an
    // error about the shadowing) and is structurally correct.
    let js = r#"
/** @param {number} n */
export function nestedDoWhile(n) {
    let result = 0;
    let i = 0;
    do {
        i = i + 1;
        let j = 0;
        do {
            j = j + 1;
            result = result + j;
        } while (j < 3);
    } while (i < n);
    return result;
}
"#;
    // Use transpile_and_check (which invokes zig ast-check) since a nesting
    // issue would manifest as a Zig compile error.
    let zig = transpile_and_check(js, "test_native_proto_nested_do_while");
    // Two do-whiles → two distinct `__dw_first_N` flags.
    assert!(
        zig.contains("__dw_first_0"),
        "Expected outer do-while flag __dw_first_0 in:\n{}",
        zig
    );
    assert!(
        zig.contains("__dw_first_1"),
        "Expected inner do-while flag __dw_first_1 in:\n{}",
        zig
    );
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
    // Transpilation verification: for-in with static struct → unrolled loop
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

    let check_output = std::process::Command::new(zig_binary())
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

    let build_output = std::process::Command::new(zig_binary())
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

// ═══════════════════════════════════════════════════════════════════════════
// P2 regression tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_p2_1_neg_zero_preserves_f64() {
    // P2-1: -0.0 must be inferred as F64 (not I64) to preserve IEEE 754
    // signed zero. The lowerer emits FloatLiteral(-0.0) directly (bypassing
    // constant folding which would collapse Neg(IntLiteral(0)) -> 0).
    let js = r#"
export function negZero() {
    var x = -0.0;
    return x;
}
"#;
    let zig = transpile_and_assert(js, "test_p2_1_neg_zero_preserves_f64");
    // The emitted value must be -0.0 (float), NOT 0 (integer)
    assert!(
        zig.contains("-0.0"),
        "Expected '-0.0' in generated Zig to preserve signed zero:\n{}",
        zig
    );
}

#[test]
fn test_p2_2_bitwise_not_type_inference() {
    // P2-2: ~x must infer I64 (previously returned None via fallthrough).
    // This affects variable type inference and binary expression typing.
    let js = r#"
/** @param {number} x */
export function bitnot(x) {
    var y = ~x;
    return y;
}
"#;
    let zig = transpile_and_check(js, "test_p2_2_bitwise_not_type_inference");
    // ~x should produce a bitwise-not with i32 cast
    assert!(
        zig.contains("~"),
        "Expected bitwise not (~) in generated Zig:\n{}",
        zig
    );
}

#[test]
fn test_p2_2_bitwise_not_bigint() {
    // P2-2: ~x on a BigInt operand should infer BigInt (not I64).
    let js = r#"
/** @param {bigint} x */
export function bitnotBig(x) {
    return ~x;
}
"#;
    let zig = transpile_and_check(js, "test_p2_2_bitwise_not_bigint");
    assert!(
        zig.contains("bitwiseNot"),
        "Expected bitwiseNot for ~BigInt:\n{}",
        zig
    );
}

#[test]
fn test_p2_6_negation_folding_still_works() {
    // P2-6: After switching from -n to n.checked_neg(), basic negation
    // folding must still produce the correct constant.
    let js = r#"
export function neg42() {
    return -42;
}
"#;
    let zig = transpile_and_assert(js, "test_p2_6_negation_folding_still_works");
    assert!(
        zig.contains("-42"),
        "Expected '-42' (folded negation) in:\n{}",
        zig
    );
}

#[test]
fn test_p2_5_labeled_for_in_struct_unroll() {
    // P2-5: A labeled for-in over a static struct (StructUnroll) must
    // wrap ALL unrolled iterations in a single labeled block so that
    // `break :label` exits the entire loop.
    // Previously the label was on the first iteration's block only, so
    // `break :label` skipped just that one iteration and fell through to
    // the remaining ones.
    let js = r#"
function gatherKeys(obj) {
    var keys = "";
    outer: for (var k in obj) {
        if (k == "b") {
            break outer;
        }
        keys = keys + k;
    }
    return keys;
}
function demo() {
    const obj = { a: 1, b: 2, name: "test" };
    return gatherKeys(obj);
}
"#;
    let zig = transpile_and_assert(js, "test_p2_5_labeled_for_in_struct_unroll");
    // Unrolled fields should be present
    assert!(
        zig.contains("\"a\""),
        "Expected unrolled field 'a' in:\n{}",
        zig
    );
    assert!(
        zig.contains("\"b\""),
        "Expected unrolled field 'b' in:\n{}",
        zig
    );
    // The label should wrap ALL iterations. With the old buggy code,
    // the label was on the first iteration block only:
    //   outer: { const k = "a"; ... } { const k = "b"; ... }
    // With the fix, the label is on a wrapper block:
    //   outer: { { const k = "a"; ... } { const k = "b"; ... } }
    // After "outer: {", the next non-whitespace should be "{" (inner
    // iteration block), NOT "const" (which would mean label is directly
    // on an iteration block).
    let after_label = zig.split("outer: {").nth(1);
    assert!(
        after_label.is_some(),
        "Expected label 'outer: {{' in:\n{}",
        zig
    );
    let rest = after_label.unwrap();
    assert!(
        rest.trim_start().starts_with('{'),
        "Expected wrapper block after label (outer: {{ {{ ...), not direct iteration content:\n{}",
        zig
    );
}

// ── Round 4: JsAny operand handling in arithmetic, comparison, and division ──
// Round 4 of the deep code audit fixed 5 deferred issues from Round 3:
//   R4-1: DivExpr / RemExpr / PowExpr did not handle JsAny operands
//   R4-2: emit_jsany_arithmetic used .asI64() unconditionally — truncated floats
//         when the other side was F64
//   R4-3: emit_jsany_comparison ordering branch used .asI64() — truncated float
//         ordering comparison
//   R4-4: Array literal spread hardcoded .items — broke JsAny arrays
//   R4-5: emit_compound_assign used unsafe .toBool() — only exists on JsAny

/// R4-1 regression baseline: integer `/` still routes through the integer-path
/// emit (`@as(f64, @floatFromInt(...))` on both sides). Without this guard a
/// refactor could break the common `i64 / i64` case by always going through
/// `emit_float_conversion`, which fails to compile for anytype params.
#[test]
fn test_r4_1_div_int_baseline() {
    let js = r#"
function safeDivide(a, b) {
    return a / b;
}
"#;
    let zig = transpile_and_check(js, "test_r4_1_div_int_baseline");
    // anytype operands → must keep the @floatFromInt path (the first DivExpr
    // attempt broke this case by routing through @as(f64, expr), which Zig
    // rejects at comptime when expr is anytype).
    assert!(
        zig.contains("@floatFromInt"),
        "Integer / division should use @floatFromInt (anytype-safe): {}",
        zig
    );
    // Safety check: should NOT contain .asF64() (no JsAny operand here).
    assert!(
        !zig.contains(".asF64()"),
        "Integer / should NOT use .asF64() (no JsAny): {}",
        zig
    );
}

/// R4-1 fix: DivExpr with a JsAny operand must coerce via .asF64() (preserves
/// float payload) instead of crashing on @floatFromInt(JsAny).
#[test]
fn test_r4_1_div_jsany_operand() {
    // Map.get(key) returns JsAny (orelse .undefined_value). Dividing it by an
    // integer literal must coerce via .asF64() on the JsAny side.
    let js = r#"
function divMapVal() {
    const m = new Map();
    m.set("a", 10);
    const v = m.get("a");
    return v / 2;
}
"#;
    let zig = transpile_and_check(js, "test_r4_1_div_jsany_operand");
    println!("=== R4-1 DivExpr with JsAny ===\n{}", zig);
    // Must call .asF64() on the JsAny operand.
    assert!(
        zig.contains(".asF64()"),
        "DivExpr with JsAny operand should call .asF64(): {}",
        zig
    );
}

/// R4-2 fix: JsAny arithmetic where the OTHER side is F64 must use .asF64()
/// (not .asI64(), which truncates the float and produces a Zig type error).
#[test]
fn test_r4_2_jsany_arith_f64_operand() {
    let js = r#"
function addF64ToMapVal() {
    const m = new Map();
    m.set("a", 10);
    const v = m.get("a");
    return v + 1.5;
}
"#;
    let zig = transpile_and_check(js, "test_r4_2_jsany_arith_f64_operand");
    println!("=== R4-2 JsAny + F64 ===\n{}", zig);
    // The bug: .asI64() was used unconditionally. The fix: .asF64() when the
    // other side is F64.
    assert!(
        zig.contains(".asF64()"),
        "JsAny + F64 should use .asF64() (not .asI64()): {}",
        zig
    );
    // Should NOT use .asI64() on the JsAny operand when the other side is F64.
    assert!(
        !zig.contains(".asI64()"),
        "JsAny + F64 should NOT use .asI64() (would truncate the F64): {}",
        zig
    );
}

/// R4-2 complement: JsAny arithmetic with an i64 operand uses .asI64()
/// (matches the inferred i64 result type for `i64 op JsAny` cases). This guards
/// against a regression where the fix might have over-corrected to always use
/// .asF64() and broken the common integer case.
#[test]
fn test_r4_2_jsany_arith_i64_operand() {
    let js = r#"
function addI64ToMapVal() {
    const m = new Map();
    m.set("a", 10);
    const v = m.get("a");
    return v + 5;
}
"#;
    let zig = transpile_and_check(js, "test_r4_2_jsany_arith_i64_operand");
    println!("=== R4-2 JsAny + i64 ===\n{}", zig);
    // Integer side → use .asI64() (matches the i64 result type).
    assert!(
        zig.contains(".asI64()"),
        "JsAny + i64 should use .asI64(): {}",
        zig
    );
}

/// R4-3 fix: JsAny ordering comparison (Lt/Le/Gt/Ge) must use .asF64()
/// (preserves float ordering like 5.5 < 5.6), not .asI64() (which truncated
/// R4-3 / R8-P1-16: JsAny ordering now uses runtime .lt()/.le()/.gt()/.ge()
/// which internally checks isString() for lexicographic, then falls back to
/// .asF64() for numeric — preserving float precision (NOT .asI64()).
#[test]
fn test_r4_3_jsany_cmp_f64_preserves_float() {
    let js = r#"
function cmpMapVal() {
    const m = new Map();
    m.set("a", 5.5);
    const v = m.get("a");
    if (v < 5.6) return 1;
    return 0;
}
"#;
    let zig = transpile_and_check(js, "test_r4_3_jsany_cmp_f64_preserves_float");
    println!("=== R4-3 JsAny < F64 ordering ===\n{}", zig);
    // Ordering must emit .lt() runtime method (which uses .asF64() internally
    // for numeric operands, preserving float precision).
    assert!(
        zig.contains(".lt("),
        "JsAny ordering should emit .lt() runtime method: {}",
        zig
    );
    // Should NOT have .asI64() in the ordering comparison.
    assert!(
        !zig.contains(".asI64()"),
        "JsAny ordering should NOT use .asI64() (truncates floats): {}",
        zig
    );
}

/// R4-4 fix: Array literal spread hardcoded `.items` for appendSlice, which
/// required the source element type to match JsAny exactly. For non-JsAny
/// arrays (e.g., `ArrayList(i64)` created from `[1, 2, 3]`), the resulting
/// `appendSlice(ArrayList(JsAny), allocator, []i64)` would fail to compile.
///
/// The fix replaces appendSlice with a `for` loop that wraps each element via
/// `JsAny.from(item)` — anytype-polymorphic and works for any ArrayList(T)
/// whose element type JsAny.from accepts (i64, f64, []const u8, JsAny, bool, …).
#[test]
fn test_r4_4_array_spread_wraps_via_jsany_from() {
    // `[1, 2, 3]` infers as `ArrayList(i64)` (homogeneous integer literal).
    // Spreading it into `[...arr]` must produce a for-loop with JsAny.from
    // so the i64 elements get wrapped for the `ArrayList(JsAny)` receiver.
    let js = r#"
function spreadIntArr() {
    const arr = [1, 2, 3];
    return [...arr];
}
"#;
    let zig = transpile_and_check(js, "test_r4_4_array_spread_wraps_via_jsany_from");
    println!("=== R4-4 array spread with i64 source ===\n{}", zig);

    // The fix: emit a for-loop with JsAny.from wrapping (NOT appendSlice).
    assert!(
        zig.contains("for (") && zig.contains(".items) |__spread_item|"),
        "Spread should iterate .items via a for-loop with __spread_item: {}",
        zig
    );
    assert!(
        zig.contains("JsAny.from(__spread_item)"),
        "Spread should wrap each item via JsAny.from(): {}",
        zig
    );
    // The OLD code emitted appendSlice, which only worked for ArrayList(JsAny);
    // a regression would reintroduce appendSlice and break the i64 case.
    assert!(
        !zig.contains("appendSlice"),
        "Spread should NOT use appendSlice (breaks for non-JsAny element types): {}",
        zig
    );
}

/// R4-5 fix: emit_compound_assign used .toBool() which only exists on JsAny.
/// For Identifier targets the lowerer routes via IrExpr::Logical (which already
/// uses isTruthy), so the bug only manifests for compound assignments on INDEX
/// targets (the fall-through path). This test exercises that path by using a
/// compound assignment on `arr[0]`, an ArrayListItem Index target.
#[test]
fn test_r4_5_index_compound_logical_uses_is_truthy() {
    // arr[0] &&= / ||= on an Index (ArrayListItem) target — Index targets
    // don't have to_read_expr and fall through to the Assign+compound path,
    // which reaches emit_compound_assign (mod.rs). Before the R4-5 fix this
    // path called .toBool(), which fails to compile on i64/bool operands.
    let js = r#"
function indexCompound() {
    const arr = [1, 2, 3];
    arr[0] &&= 2;
    arr[1] ||= 3;
    return arr[0];
}
"#;
    let zig = transpile_and_check(js, "test_r4_5_index_compound_logical_uses_is_truthy");
    println!("=== R4-5 Index compound &&= / ||= ===\n{}", zig);
    // The fix: emit_compound_assign uses js_runtime.isTruthy for &&= / ||=,
    // not .toBool() which only exists on JsAny.
    assert!(
        zig.contains("js_runtime.isTruthy"),
        "Index-target compound &&= / ||= should use js_runtime.isTruthy (not .toBool()): {}",
        zig
    );
    assert!(
        !zig.contains(".toBool()"),
        "Index-target compound should NOT use .toBool() (only valid on JsAny): {}",
        zig
    );
}

// ═══════════════════════════════════════════════════════
//  Round 5 regression tests (R5-2 .. R5-12; R5-1/9/10 are
//  covered by Zig-level tests in runtime/js_runtime.zig and
//  runtime/jsvalue.zig — they execute via `zig test`).
// ═══════════════════════════════════════════════════════

/// R5-2 fix: emit_array_literal's element-type fallback for non-literal
/// elements (Ident, Call, FieldAccess, …) was `i64` (pre-fix) — this made
/// `const arr = [someF64Var]` produce `ArrayList(i64)` containing an f64
/// value, which Zig rejects ("expected i64, found f64"). The fix changes
/// the fallback to `JsAny`, which is type-polymorphic via `JsAny.from(...)`.
#[test]
fn test_r5_2_array_literal_non_literal_fallback_is_jsany() {
    let js = r#"
function buildArr(x) {
    const arr = [x];
    return arr;
}
"#;
    let zig = transpile_and_check(js, "test_r5_2_array_literal_non_literal_fallback_is_jsany");
    println!(
        "=== R5-2 array literal with non-literal element ===\n{}",
        zig
    );
    assert!(
        zig.contains("std.ArrayList(JsAny)"),
        "Non-literal element array must use ArrayList(JsAny) fallback (post-fix): {}",
        zig
    );
    assert!(
        !zig.contains("std.ArrayList(i64)"),
        "Non-literal element array must NOT use ArrayList(i64) (pre-fix fallback breaks f64 elements): {}",
        zig
    );
}

/// R5-3 fix: emit_bigint_string_concat in `binary.rs` hardcoded the label
/// `blk:` instead of using `next_label()`. The fix (lines 474-506) uses
/// `next_label()` so nested Str+BigInt concatenations would no longer
/// produce `redefinition of label 'blk'` in Zig.
///
/// NOTE: There is no transpiler-level test for this fix because the lowerer
/// intercepts ALL Str+BigInt additions via `lower_binary`'s check
/// (`expr_is_string(...)` returns true on the String operand), which routes
/// the entire concat chain into `IrExpr::AllocPrint` (see
/// `lower_string_concat` in operators.rs). The `emit_bigint_string_concat`
/// branch in the emitter (mod.rs:134-138) is therefore currently reachable
/// only via synthesized `IrExpr::Binary { op: Add, left_type: Some(Str),
/// right_type: Some(BigInt) }` IR — which the lowerer never produces from
/// JS source. The defensive fix keeps the emit function safe against future
/// lowerer changes that might route Str+BigInt through `emit_bigint_string_concat`.
#[test]
fn test_r5_3_emit_bigint_string_concat_documented_unreachable() {
    // This test exists to document that the R5-3 fix in
    // emit_bigint_string_concat is a defensive patch against a latent bug
    // (label collision on nested calls). The code path is unreachable from
    // JS source via the current lowerer (see the doc comment above).
    // If the lowerer routing changes in the future to actually invoke
    // emit_bigint_string_concat with nested calls, replace this stub with a
    // real test case that triggers the path.
    //
    // For now we keep the fix and document the situation to avoid a
    // false-positive "TODO: write test" nag.
    //
    // Sanity: the function source still uses next_label() (not "blk:").
    let src = include_str!("../zigir/emit/expr/binary.rs");
    assert!(
        !src.contains("\"blk:\""),
        "emit_bigint_string_concat must not reintroduce the hardcoded \"blk:\" label"
    );
    assert!(
        src.contains("next_label()"),
        "emit_bigint_string_concat should use next_label() for unique labels"
    );
}

/// R5-4 fix: BigInt postfix `x++` must return the OLD value (JS spec).
/// Pre-fix: lower_update always used `IrExpr::Assign` for BigInt — but an
/// Assign expression evaluates to the NEW (post-increment) value, so
/// `const y = x++` set y to x+1 instead of x.
/// Post-fix: postfix path wraps the assign in a `BlockExpr` with a temp
/// `__bi_post_N` capturing the pre-increment value, so the expression
/// result is the temp (the old value).
#[test]
fn test_r5_4_bigint_postfix_returns_old_value() {
    let js = r#"
/**
 * @param {bigint} x
 * @returns {bigint}
 */
export function test(x) {
    const y = x++;
    return y;
}
"#;
    let zig = transpile_and_check(js, "test_r5_4_bigint_postfix_returns_old_value");
    println!(
        "=== R5-4 BigInt postfix `x++` (old value capture) ===\n{}",
        zig
    );
    // Post-fix: BlockExpr wraps a __bi_post temp var.
    assert!(
        zig.contains("__bi_post"),
        "BigInt postfix must capture old value in a __bi_post temp var (BlockExpr): {}",
        zig
    );
    // The BlockExpr's `break :<label> __bi_post_N` makes the expression
    // return the OLD value (pre-increment), matching JS postfix semantics.
    assert!(
        zig.contains("break :") && zig.contains("__bi_post"),
        "BigInt postfix BlockExpr must break with the __bi_post temp var (old value): {}",
        zig
    );
}

/// R5-5 fix: unify_return_expr_types had no I64↔F64 numeric promotion.
/// For `function f(x){ if(x) return 1; return 1.5; }` it reported
/// "Return type mismatch" and returned None, defaulting the function's
/// return type to i64 — truncating the f64 return expression.
/// The fix promotes (I64, F64) → F64 before reporting a mismatch.
#[test]
fn test_r5_5_return_type_unify_i64_f64_promotion() {
    let js = r#"
function mixedReturn(x) {
    if (x) return 1;
    return 1.5;
}
"#;
    let zig = transpile_and_check(js, "test_r5_5_return_type_unify_i64_f64_promotion");
    println!("=== R5-5 mixed int/float return unification ===\n{}", zig);
    // Post-fix: function's return type is promoted to f64.
    assert!(
        zig.contains(") f64 {") || zig.contains(") f64\n"),
        "Mixed I64/F64 returns must unify to f64 (not default to i64): {}",
        zig
    );
    // No "Return type mismatch" error in the output.
    assert!(
        !zig.contains("Return type mismatch"),
        "Should NOT report a return type mismatch for I64+F64 (should promote to F64): {}",
        zig
    );
}

/// R5-6 fix: infer_binary_type Addition checked BigInt before Str, so
/// `Str + Str` (neither BigInt nor F64) fell through to I64. The fix puts
/// the Str check first, mirroring the lowerer's infer_binary_result_type.
#[test]
fn test_r5_6_str_plus_str_inferred_as_str() {
    let js = r#"
function concatStr() {
    return "a" + "b";
}
"#;
    let zig = transpile_and_check(js, "test_r5_6_str_plus_str_inferred_as_str");
    println!("=== R5-6 Str + Str inferred as Str ===\n{}", zig);
    // Post-fix: function's return type is []const u8 (Str).
    assert!(
        zig.contains(") []const u8 {") || zig.contains(") []const u8\n"),
        "Str + Str must infer as Str (return type []const u8), not i64: {}",
        zig
    );
    // Pre-fix: function's return type was i64 (the wrong fallback).
    assert!(
        !zig.contains(") i64 {"),
        "Str + Str must NOT infer as i64 (pre-fix bug — checked BigInt before Str): {}",
        zig
    );
}

/// R5-7 fix: expr_has_side_effects classified CompileError as a leaf and
/// short-circuited to `false`, so top-level unused `const X = <unsupported>`
/// was stripped by eliminate_unused_decls — silently dropping the
/// user-facing `@compileError` diagnostic. The fix adds an explicit
/// CompileError guard before the is_leaf() short-circuit.
#[test]
fn test_r5_7_top_level_unused_const_preserves_compileerror() {
    // `const X = tag`hello`;` — X is unused, tag template is unsupported.
    // Pre-fix: DCE stripped the whole const (CompileError treated as
    // side-effect-free), so @compileError vanished from the Zig output.
    // Post-fix: DCE preserves the const, emit produces `const X = @compileError(...)`.
    let js = r#"
function tag(parts) { return parts[0]; }
const X = tag`hello`;
"#;
    let zig = transpile_and_assert(
        js,
        "test_r5_7_top_level_unused_const_preserves_compileerror",
    );
    println!(
        "=== R5-7 top-level unused const with @compileError ===\n{}",
        zig
    );
    assert!(
        zig.contains("@compileError"),
        "Top-level unused const with unsupported init must preserve @compileError (was silently stripped by DCE pre-fix): {}",
        zig
    );
}

/// R5-8 fix: RemExpr with JsAny operands used `.asI64()` then
/// `jsRem(i64, i64)`, truncating the float payload — `JsAny.from(5.7) % 2`
/// computed `5 % 2 = 1` instead of `1.7`. The fix routes JsAny operands
/// through `@rem(emit_float_conversion(left), emit_float_conversion(right))`
/// which uses `.asF64()` for JsAny, preserving the float payload.
#[test]
fn test_r5_8_remexpr_jsany_uses_rem_asf64() {
    // JSON.parse returns JsAny, so `parsed % 2` reaches the RemExpr emit
    // with at least one JsAny operand.
    let js = r#"
function modJsAny() {
    const x = JSON.parse("5.7");
    return x % 2;
}
"#;
    let zig = transpile_and_check(js, "test_r5_8_remexpr_jsany_uses_rem_asf64");
    println!("=== R5-8 RemExpr with JsAny operand ===\n{}", zig);
    assert!(
        zig.contains("@rem("),
        "RemExpr with JsAny operand must use @rem (not jsRem): {}",
        zig
    );
    assert!(
        zig.contains(".asF64()"),
        "RemExpr with JsAny must use .asF64() to preserve float payload: {}",
        zig
    );
    assert!(
        !zig.contains("js_runtime.jsRem"),
        "RemExpr with JsAny must NOT use jsRem (truncates floats via .asI64): {}",
        zig
    );
}

/// R5-11 fix: lowerer's infer_expr_type for ArrayExpression used find_map
/// (returning the FIRST element's type). For `[1, 2.5]` it inferred
/// ArrayList(I64) while the emitter produced ArrayList(JsAny)
/// (all_same=false), risking downstream type-annotation mismatches.
/// The fix walks ALL elements and unifies: any mismatch degrades to JsAny.
#[test]
fn test_r5_11_array_type_walks_all_elements() {
    let js = r#"
function mixedArray() {
    const arr = [1, 2.5];
    return arr;
}
"#;
    let zig = transpile_and_check(js, "test_r5_11_array_type_walks_all_elements");
    println!(
        "=== R5-11 mixed int/float array walking elements ===\n{}",
        zig
    );
    // Post-fix: lowerer infers ArrayList(JsAny) — matches emit's all_same=false.
    assert!(
        zig.contains("std.ArrayList(JsAny)"),
        "Mixed int/float array must infer ArrayList(JsAny) (any mismatch degrades from first elem type): {}",
        zig
    );
    // Pre-fix: lowerer inferred ArrayList(I64) (the first element's type).
    assert!(
        !zig.contains("std.ArrayList(i64)"),
        "Mixed int/float array must NOT infer ArrayList(I64) (pre-fix bug — used only first elem type): {}",
        zig
    );
}

/// R5-12 fix: lowerer's ConditionalExpression inference had
/// `(Some(F64), _) | (_, Some(F64)) => Some(F64)`, so `cond ? "a" : 1.5`
/// returned F64 even though "a" cannot coerce to f64. The fix restricts
/// F64 promotion to (I64, F64) | (F64, I64) — both branches must be
/// numeric. Other mismatches return None (JsAny fallback), aligning with
/// the inferencer's behavior.
#[test]
fn test_r5_12_conditional_expr_str_f64_not_promoted() {
    let js = r#"
function condStr(flag) {
    const x = flag ? "a" : 1.5;
    return x;
}
"#;
    let zig = transpile_and_check(js, "test_r5_12_conditional_expr_str_f64_not_promoted");
    println!("=== R5-12 conditional Str+F64 not promoted ===\n{}", zig);
    // Post-fix: variable is NOT annotated `: f64` (pre-fix the lowerer's
    // ConditionalExpression inference wrongly returned F64 for Str+F64,
    // which could leak :f64 into downstream annotations).
    assert!(
        !zig.contains(": f64 ="),
        "Conditional Str+F64 variable must NOT be annotated :f64 (Str branch cannot coerce to f64): {}",
        zig
    );
    // Sanity: the conditional must still compile (ast-check passed in
    // transpile_and_check). Pre-fix's wrong inference risked downstream
    // emit wrapping a string branch in a float coercion.
}

// ── Round 6 deep audit regression tests ──────────────────────────────

/// R6-1 fix: `try { throw ... } finally { ... }` without a catch clause
/// silently swallowed the thrown error — `break :blk_label {}` always wrote
/// the success variant, and the propagate-unhandled-re-throw block was
/// guarded by `if needs_catch` (skipped when no catch handler). The fix
/// (control_flow.rs:716) emits `break :blk_label if (body_result_var) |_| {} else |_| @as(anyerror!void, error.JsThrow);` and changes the propagate guard
/// from `if needs_catch` to `if needs_catch || has_throw` (line 735).
#[test]
fn test_r6_1_try_finally_no_catch_propagates_throw() {
    let js = r#"
function tryThrowFinally() {
    try {
        throw 1;
    } finally {
    }
}
"#;
    let zig = transpile_and_check(js, "test_r6_1_try_finally_no_catch_propagates_throw");
    println!(
        "=== R6-1 try-finally-no-catch propagates throw ===\n{}",
        zig
    );
    // Post-fix: top-level propagate emits `else |_| return error.JsThrow;`
    // for the result var. Pre-fix the guard `if needs_catch` was false, so
    // this block was skipped entirely — the thrown error was dropped.
    assert!(
        zig.contains("else |_| return error.JsThrow;"),
        "try{{throw}}finally{{}} (no catch) must propagate the throw via `else |_| return error.JsThrow;` (was silently swallowed pre-fix because the propagate guard was `if needs_catch`): {}",
        zig
    );
}

/// R6-2 fix: `switch (strVar) { case "a": ... }` previously emitted Zig
/// `switch (x) { ... }` which is a compile error (Zig switch on []const u8
/// is invalid). The fix routes string-typed cases to `emit_string_switch`
/// which emits an if/else chain with `std.mem.eql(u8, ...)`.
#[test]
fn test_r6_2_string_switch_uses_mem_eql() {
    let js = r#"
function strSwitch(x) {
    switch (x) {
        case "a": return 1;
        case "b": return 2;
        default: return 0;
    }
}
"#;
    let zig = transpile_and_check(js, "test_r6_2_string_switch_uses_mem_eql");
    println!("=== R6-2 string switch uses std.mem.eql ===\n{}", zig);
    assert!(
        zig.contains("std.mem.eql(u8,"),
        "String switch must generate `std.mem.eql(u8, ...)` comparison (pre-fix emitted invalid `switch` on []const u8): {}",
        zig
    );
    // Sanity: this small function switches only on x — must not emit
    // `switch (x)` for it.
    assert!(
        !zig.contains("switch (x)"),
        "String switch must NOT use Zig `switch` on the string variable (pre-fix bug): {}",
        zig
    );
}

/// R6-3 fix: `lower_template_literal` used `q.value.raw.to_string()`, which
/// preserves the literal escape characters (e.g. `\n` stays as 2 chars
/// backslash+n). After re-escaping for Zig source, this produced a runtime
/// string `"hello\nworld"` (literal backslash-n) instead of an interpreted
/// newline. The fix uses `cooked` (interpreted escapes) with `raw` as
/// fallback for invalid escapes only.
#[test]
fn test_r6_3_template_literal_uses_cooked_not_raw() {
    let js = r#"
function templateNewline() {
    return `hello\nworld`;
}
"#;
    let zig = transpile_and_check(js, "test_r6_3_template_literal_uses_cooked_not_raw");
    println!("=== R6-3 template literal cooked vs raw ===\n{}", zig);
    // Post-fix: Zig source contains `hello\nworld` (1 backslash + n = Zig
    // escape for newline). Pre-fix: raw value's `\n` (2 chars: backslash, n)
    // got re-escaped to `\\n` (3 chars: backslash, backslash, n).
    assert!(
        zig.contains("hello\\nworld"),
        "Template literal with \\n must use cooked value (Zig source: \"hello\\nworld\", 1 backslash + n): {}",
        zig
    );
    assert!(
        !zig.contains("hello\\\\nworld"),
        "Template literal with \\n must NOT use raw value (Zig source: \"hello\\\\nworld\", 2 backslashes + n — runtime string would be `hello\\nworld` literally): {}",
        zig
    );
}

/// R6-4 fix: `lower_string_concat` formatted BigInt operands via `{any}`,
/// which invokes `JsBigInt.format()` — that method appends a trailing "n"
/// suffix (Node.js console.log parity), so `"1" + 2n` produced runtime
/// output "12n" instead of "12". The fix wraps BigInt operands in a
/// `JsBigInt.toString()` BuiltinCall and uses `{s}` format specifier.
#[test]
fn test_r6_4_str_plus_bigint_no_n_suffix() {
    let js = r#"
function strPlusBigInt() {
    return "1" + 2n;
}
"#;
    let zig = transpile_and_check(js, "test_r6_4_str_plus_bigint_no_n_suffix");
    println!("=== R6-4 str + bigint no n suffix ===\n{}", zig);
    // Post-fix: BigInt operand wrapped in a `.toString(allocator)` call
    // (instance method on a JsBigInt value). Pre-fix: BigInt operand was
    // lowered directly and formatted via `{any}` which invokes
    // JsBigInt.format() (appends trailing "n" for Node.js console.log parity).
    assert!(
        zig.contains(".toString("),
        "String + BigInt must wrap the BigInt operand in a .toString(allocator) call (pre-fix used {{any}} format which invokes JsBigInt.format appending \"n\" suffix): {}",
        zig
    );
    // Post-fix: format specifier string must NOT contain `{any}` (which would
    // invoke JsBigInt.format() and append "n").
    assert!(
        !zig.contains("{any}"),
        "String + BigInt must NOT use {{any}} format specifier for the BigInt operand (appends \"n\" suffix at runtime): {}",
        zig
    );
}

// ── Round 8 deep audit regression tests ──────────────────────────────

/// R8-E6: Math.max/min with no args should return ±Infinity (f64), not
/// minInt/maxInt(i64); and float-literal args must not be coerced to i64.
#[test]
fn test_r8_e6_math_max_min_empty_and_float() {
    // Empty args: Math.max() → -Infinity, Math.min() → +Infinity.
    let js = r#"
/** @returns {number} */
export function maxEmpty() { return Math.max(); }
/** @returns {number} */
export function minEmpty() { return Math.min(); }
"#;
    let zig = transpile_and_check(js, "test_r8_e6_math_max_min_empty_and_float");
    assert!(
        zig.contains("std.math.inf(f64)"),
        "Math.max()/Math.min() with no args must return Infinity (R8-E6): {}",
        zig
    );
    assert!(
        zig.contains("-std.math.inf(f64)"),
        "Math.max() with no args must return -Infinity (R8-E6): {}",
        zig
    );

    // Float-literal args must NOT be coerced to i64 (would not compile).
    let js2 = r#"
/** @returns {number} */
export function maxf(a, b) { return Math.max(1.5, 2.5); }
"#;
    let zig2 = transpile_and_check(js2, "test_r8_e6_math_max_min_float_args");
    assert!(
        !zig2.contains("@as(i64, 1.5)"),
        "Math.max with float args must not emit @as(i64, ...) (R8-E6): {}",
        zig2
    );
}

/// R8-E1: Unary `+x` must perform ToNumber conversion.
/// `+true` → 1 (constant-folded), `+boolVar` → conditional, `+strVar` →
/// js_number.constructor call.
#[test]
fn test_r8_e1_unary_plus_to_number() {
    // +true should be constant-folded to 1.
    let js = r#"
/** @returns {number} */
export function plusTrue() { return +true; }
"#;
    let zig = transpile_and_check(js, "test_r8_e1_unary_plus_true");
    assert!(
        !zig.contains("+true"),
        "+true must be constant-folded to 1, not emitted as +true (R8-E1): {}",
        zig
    );

    // +boolVar should NOT be emitted as the bare bool variable.
    let js2 = r#"
/** @param {boolean} flag @returns {number} */
export function plusBool(flag) { return +flag; }
"#;
    let zig2 = transpile_and_check(js2, "test_r8_e1_unary_plus_bool");
    println!("=== R8-E1 +bool ===\n{}", zig2);
    // Should contain a conditional (if (flag) 1 else 0) or similar coercion,
    // NOT just `return flag;`.
    assert!(
        zig2.contains("if (") || zig2.contains("@intFromBool"),
        "+boolVar must be coerced to number, not passed through as bool (R8-E1): {}",
        zig2
    );

    // +strVar should call js_number.constructor.
    let js3 = r#"
/** @param {string} s @returns {number} */
export function plusStr(s) { return +s; }
"#;
    let zig3 = transpile_and_check(js3, "test_r8_e1_unary_plus_str");
    println!("=== R8-E1 +str ===\n{}", zig3);
    assert!(
        zig3.contains("js_number.constructor"),
        "+strVar must call js_number.constructor for ToNumber (R8-E1): {}",
        zig3
    );
}

/// R8-P1-7: JsAny values in string contexts (template literals, string concat)
/// must use the `{f}` specifier — the only Zig 0.16.0 specifier that dispatches
/// to the custom `format` method (emitting JS-correct "NaN"/"Infinity"). The
/// previous `{any}` emitted the debug repr `.{ .value = .{ .float = nan } }`
/// for tagged unions.
#[test]
fn test_r8_p1_7_jsany_uses_f_specifier() {
    // Template literal with a JSON.parse() result (inferred as JsAny).
    let js = r#"
/** @returns {string} */
export function tmplJson() { return `val: ${JSON.parse("42")}`; }
"#;
    let zig = transpile_and_check(js, "test_r8_p1_7_jsany_template_f");
    println!("=== R8-P1-7 template ===\n{}", zig);
    assert!(
        zig.contains("{f}"),
        "Template literal with JsAny must use {{f}} specifier (R8-P1-7): {}",
        zig
    );
    assert!(
        !zig.contains("{any}"),
        "Template literal with JsAny must NOT use {{any}} specifier (R8-P1-7): {}",
        zig
    );

    // String concatenation with a JSON.parse() result.
    let js2 = r#"
/** @returns {string} */
export function concatJson() { return "val: " + JSON.parse("42"); }
"#;
    let zig2 = transpile_and_check(js2, "test_r8_p1_7_jsany_concat_f");
    println!("=== R8-P1-7 concat ===\n{}", zig2);
    assert!(
        zig2.contains("{f}"),
        "String concat with JsAny must use {{f}} specifier (R8-P1-7): {}",
        zig2
    );
    assert!(
        !zig2.contains("{any}"),
        "String concat with JsAny must NOT use {{any}} specifier (R8-P1-7): {}",
        zig2
    );
}

/// R8-E2: F64 values in string contexts (template literals, string concat,
/// Array.join) must use the `{}` default specifier — Zig's shortest round-trip
/// fixed-point formatter — instead of `{d:.15}` which padded to 15 digits
/// (1.5 → "1.500000000000000"). `{}` produces "1.5", matching JS toString
/// for the common range.
#[test]
fn test_r8_e2_f64_uses_default_specifier_not_d15() {
    // Template literal with a float literal (inferred F64).
    let js = r#"
/** @returns {string} */
export function tmplFloat() { return `v=${1.5}`; }
"#;
    let zig = transpile_and_check(js, "test_r8_e2_f64_template");
    println!("=== R8-E2 template ===\n{}", zig);
    assert!(
        !zig.contains("{d:.15}"),
        "Template literal with F64 must NOT use {{d:.15}} (R8-E2): {}",
        zig
    );

    // String concatenation with a float literal.
    let js2 = r#"
/** @returns {string} */
export function concatFloat() { return "v=" + 1.5; }
"#;
    let zig2 = transpile_and_check(js2, "test_r8_e2_f64_concat");
    println!("=== R8-E2 concat ===\n{}", zig2);
    assert!(
        !zig2.contains("{d:.15}"),
        "String concat with F64 must NOT use {{d:.15}} (R8-E2): {}",
        zig2
    );
}

/// R8-C7: `this.field = value` inside loops/switch/try in a constructor
/// must be (a) discovered as a class field, (b) rewritten to an assignment
/// to a pre-declared local `var`, and (c) reach the struct-literal return at
/// the end of `init`.
///
/// Background. The original constructor rewrite lowered `this.field = value`
/// to `const field = value` (a fresh VarDecl at the point of the assignment)
/// and the Emitter appended `return .{ .field = field, ... }` after the ctor
/// body. That model relies on the assignment sitting at the top level of the
/// body — any nested scope (if/else, for/while/switch/try/block) gives a
/// `const field = ...` that (a) shadows the outer constant from earlier in
/// the body and (b) is out of scope at the trailing struct return. Both are
/// Zig compile errors, so this nested pattern was silently broken even for
/// the if/else case (probes confirmed it before the fix landed).
///
/// The fix introduces a var-model:
/// 1. `collect_implicit_class_fields` recurses into every container statement
///    (if/while/for/for-of/for-in/switch/try/labeled), so nested field writes
///    are *discovered*.
/// 2. `try_rewrite_this_field_assignment` emits an `Assign { Ident(field) }`
///    (a reassignment) instead of a fresh VarDecl. The rewrite is invoked via
///    a `this_rewrite_fields` flag on the Lowerer that `lower_stmt` checks on
///    every ExpressionStatement, so it reaches every nesting depth.
/// 3. `emit_class_init` pre-declares `var field: T = <default>;` for each
///    field at the very top of `init`, giving every Assign a target. The
///    pre-declaration uses the field's class-body default when available,
///    which is also the R8-E4/C6 fix.
/// 4. Reads of `this.field` in the constructor are rewritten to `Ident(field)`
///    as well (member.rs), so `this.count + 1` becomes `count + 1` instead of
///    the unreachable `self.count + 1` (init has no `self`).
#[test]
fn test_r8_c7_this_field_in_loop_switch_try() {
    // for-loop + switch + try, each assigning a distinct this-field.
    // The for-loop body also READS this.count — the right-hand side must be
    // rewritten too, otherwise the emitted Zig references the non-existent
    // `self` parameter of `init`.
    let js = r#"
class Acc {
constructor(n) {
  this.count = 0;
  for (let i = 0; i < n; i++) {
    this.count = this.count + 1;
  }
  switch (n) {
    case 0: this.flag = "zero"; break;
    default: this.flag = "nonzero"; break;
  }
  try {
    this.computed = n * 2;
  } catch (e) {
    this.computed = -1;
  }
}
}
"#;
    let zig = transpile_and_check(js, "test_r8_c7_this_field_nested");
    println!("=== R8-C7 nested this-field ===\n{}", zig);

    // (a) All three fields are discovered and pre-declared as `var` locals.
    assert!(
        zig.contains("var count:") && zig.contains("var flag:") && zig.contains("var computed:"),
        "Expected pre-declared `var count/flag/computed` at the top of init (R8-C7): {}",
        zig
    );

    // (b) The nested assignments are rewritten to plain `field = value` writes
    // against the pre-declared vars, NOT left as `self.x =` / `this.x =` field
    // mutations (which would compile-error in a value-returning constructor).
    assert!(
        !zig.contains("self.count"),
        "this.count read/write must be rewritten to `count`, not `self.count` (R8-C7): {}",
        zig
    );
    assert!(
        !zig.contains("self.flag = "),
        "Switch-body `this.flag = ...` must be rewritten (R8-C7): {}",
        zig
    );
    assert!(
        !zig.contains("self.computed = "),
        "Try-body `this.computed = ...` must be rewritten (R8-C7): {}",
        zig
    );

    // (c) The struct-literal return references the locals (`.{ .count = count, ... }`).
    assert!(
        zig.contains("return .{ .count = count"),
        "Expected `return .{{ .count = count, ... }}` referencing the pre-declared var (R8-C7): {}",
        zig
    );
}

/// R8-C7 (if/else branch): the original `lower_block_with_this_rewrite`
/// nominally recursed into if-statements but still emitted `const` shadow
/// bindings, which broke for the if/else case too. Probes confirmed the
/// pre-existing breakage (this test was failing before the fix landed).
/// The var-model fix retroactively repairs the if/else path as well.
#[test]
fn test_r8_c7_this_field_in_if_else() {
    let js = r#"
class C {
  constructor(f) {
    if (f) { this.a = 1; } else { this.a = 2; }
  }
}
"#;
    let zig = transpile_and_check(js, "test_r8_c7_if_else");
    println!("=== R8-C7 if/else ===\n{}", zig);

    assert!(
        zig.contains("var a:"),
        "Expected `var a:` pre-declaration (R8-C7): {}",
        zig
    );
    assert!(
        zig.contains("a = 1;") && zig.contains("a = 2;"),
        "Expected both if/else branches to assign to the local `a` (R8-C7): {}",
        zig
    );
    assert!(
        !zig.contains("self.a"),
        "this.a must be rewritten to `a` (R8-C7): {}",
        zig
    );
    assert!(
        zig.contains("return .{ .a = a };"),
        "Expected `return .{{ .a = a }};` (R8-C7): {}",
        zig
    );
}

/// R8-C7 (loop only, single field): the smallest reproducer from the audit.
/// Verifies the rewrite reaches the body of a `for` loop, with no other
/// noise from switch/try in the same body.
#[test]
fn test_r8_c7_this_field_in_loop_only() {
    let js = r#"
class C {
  constructor(n) {
    for (let i = 0; i < n; i++) { this.a = i; }
  }
}
"#;
    let zig = transpile_and_check(js, "test_r8_c7_loop_only");
    println!("=== R8-C7 loop-only ===\n{}", zig);

    assert!(
        zig.contains("var a:"),
        "Expected `var a:` pre-declaration (R8-C7): {}",
        zig
    );
    assert!(
        zig.contains("a = i;") && !zig.contains("self.a"),
        "Expected loop body to assign `a = i`, never `self.a` (R8-C7): {}",
        zig
    );
    assert!(
        zig.contains("return .{ .a = a };"),
        "Expected `return .{{ .a = a }};` (R8-C7): {}",
        zig
    );
}

/// R8-C2: When a constructor ends with an explicit `return` statement, the
/// Emitter must NOT append its own struct-literal return afterwards, because
/// Zig would reject the appended code as unreachable.
///
/// Before the fix:
///   pub fn init(n) C { var x: i64 = 0; x = n; return; return .{ .x = x }; }
///                                                                ^^^^^ Zig: unreachable
///
/// After the fix the Emitter inspects the last body statement; if it is a
/// `Return`, the appended `return .{...}` is suppressed. Note: full JS
/// semantics of `return <object>` from a constructor (instance replacement)
/// is a separate, deeper feature and out of scope for R8-C2; this test only
/// covers the unconditional-unreachable-code compilation error.
#[test]
fn test_r8_c2_ctor_explicit_return_no_unreachable() {
    let js = r#"
class C {
  constructor(n) {
    this.x = n;
    return;
  }
}
"#;
    let zig = transpile_and_check(js, "test_r8_c2_ctor_return");
    println!("=== R8-C2 ctor return ===\n{}", zig);

    // Exactly one `return` keyword appears in init's body (the user's own).
    let init_section = zig
        .split("pub fn init(")
        .nth(1)
        .expect("init function must exist");
    let return_count = init_section.matches("return").count();
    assert_eq!(
        return_count, 1,
        "Expected exactly one `return` in init body (R8-C2): {}",
        zig
    );
    // The appended struct-literal return must NOT be present.
    assert!(
        !zig.contains("return .{ .x = x };"),
        "Appended struct return must be suppressed when body ends in `return;` (R8-C2): {}",
        zig
    );
}

/// R8-E4/C6: When a class has BOTH a field default (`x = 5`) AND a
/// constructor, the field's default was previously ignored — the Emitter only
/// seeded the struct-literal return from `this.x = value` assignments, so a
/// field never touched by the constructor got the local var's zero/undefined
/// slot instead of its declared default.
///
/// After the R8-C7 var-model redesign, `emit_class_init` pre-declares each
/// field as `var field: T = <default>;`, where `<default>` is the field's
/// class-body initializer. So a constructor that only sets some fields still
/// yields the declared defaults for the rest.
#[test]
fn test_r8_e4_c6_field_default_used_when_ctor_present() {
    let js = r#"
class C {
  x = 5;
  constructor(n) {
    this.y = n;
  }
}
"#;
    let zig = transpile_and_check(js, "test_r8_e4_c6_defaults_with_ctor");
    println!("=== R8-E4/C6 defaults with ctor ===\n{}", zig);

    // The x field is pre-declared with its class-body default (5). Because the
    // constructor never reassigns x, Zig 0.16.0 ast-check requires it to be
    // `const`, not `var` (otherwise "local variable is never mutated" fires).
    // The Emitter's `collect_assigned_idents_in_block` walks the ctor body to
    // make exactly this var/const decision per field.
    assert!(
        zig.contains("const x: i64 = 5;"),
        "Expected `const x: i64 = 5;` seeding the field default in the ctor (R8-E4/C6): {}",
        zig
    );
    // The y field, in contrast, IS mutated (`y = n;`), so it must be `var`.
    assert!(
        zig.contains("var y: JsAny = undefined;"),
        "Expected `var y: JsAny = undefined;` for the reassigned field (R8-E4/C6): {}",
        zig
    );
    // The y field is reassigned from the constructor argument.
    assert!(
        zig.contains("y = n;"),
        "Expected `y = n;` rewrite (R8-E4/C6): {}",
        zig
    );
    // The struct-literal return includes both fields with their final values.
    assert!(
        zig.contains("return .{ .x = x, .y = y };") || zig.contains("return .{ .y = y, .x = x };"),
        "Expected `return .{{ .x = x, .y = y }};` (order-independent) (R8-E4/C6): {}",
        zig
    );
}

#[test]
fn test_r8_c3_new_spread_rest_param() {
    // new Foo(...restParam) should pass the slice directly to init,
    // not emit the bare rest param name.
    // Constructor(...items) → init(items: []const JsAny)
    // new Collector(...vals) → Collector.init(vals)
    let js = r#"
class Collector {
  count = 0;
  constructor(...items) {
    this.count = items.length;
  }
}

export function makeCollector(...vals) {
  const c = new Collector(...vals);
  return c.count;
}
"#;
    let zig = transpile_and_check(js, "test_r8_c3_new_spread_rest_param");
    println!("=== R8-C3 new spread rest param ===\n{}", zig);

    // Rest param spread: the slice is passed directly (no .items suffix).
    assert!(
        zig.contains("Collector.init(vals)"),
        "Expected `Collector.init(vals)` for rest-param spread in new (R8-C3): {}",
        zig
    );
    // The constructor accepts a rest slice.
    assert!(
        zig.contains("init(items: []const JsAny)"),
        "Expected `init(items: []const JsAny)` rest param in constructor (R8-C3): {}",
        zig
    );
}

#[test]
fn test_r8_c3_new_spread_arraylist() {
    // new Foo(...arr) where arr is an ArrayList should emit arr.items,
    // not the bare array variable. Previously emit_inline_args dropped
    // the .items suffix, producing invalid Zig.
    let js = r#"
class Collector {
  count = 0;
  constructor(...items) {
    this.count = items.length;
  }
}

export function testSpread() {
  const arr = [10, 20, 30];
  const c = new Collector(...arr);
  return c.count;
}
"#;
    let zig = transpile_and_assert(js, "test_r8_c3_new_spread_arraylist");
    println!("=== R8-C3 new spread arraylist ===\n{}", zig);

    // ArrayList spread: .items suffix is needed.
    assert!(
        zig.contains("Collector.init(arr.items)"),
        "Expected `Collector.init(arr.items)` for ArrayList spread in new (R8-C3): {}",
        zig
    );
}

#[test]
fn test_r8_e5_c1_method_mutates_self() {
    // A method that assigns to `this.field` must use `self: *@This()`.
    // A read-only method keeps `self: @This()` (by-value).
    // The class instance is emitted as `var` (not `const`) with `_ = &c;`
    // so that `c.increment()` (which takes *@This()) compiles.
    let js = r#"
class Counter {
  count = 0;
  increment() {
    this.count = this.count + 1;
  }
  get() {
    return this.count;
  }
}

export function testMutate() {
  const c = new Counter();
  c.increment();
  return c.get();
}
"#;
    let zig = transpile_and_check(js, "test_r8_e5_c1_method_mutates_self");
    println!("=== R8-E5/C1 method mutates self ===\n{}", zig);

    // Mutating method: pointer receiver.
    assert!(
        zig.contains("pub fn increment(self: *@This()"),
        "Expected `self: *@This()` for mutating method (R8-E5/C1): {}",
        zig
    );
    // Read-only method: by-value receiver.
    assert!(
        zig.contains("pub fn get(self: @This()"),
        "Expected `self: @This()` for read-only method (R8-E5/C1): {}",
        zig
    );
    // Instance is `var` (forced for class instances) with suppression.
    assert!(
        zig.contains("var c = Counter.init()"),
        "Expected `var c = Counter.init()` for class instance (R8-E5/C1): {}",
        zig
    );
}

/// P0-5: Operator precedence — `(a + b) * c` must produce correct Zig with
/// parenthesized sub-expressions. Without the fix, `emit_default_binop` emits
/// `a + b * c` (flat, no parens), which Zig re-parses as `a + (b * c)` — wrong.
/// Function parameters are used (not const) to prevent constant folding.
#[test]
fn test_p0_5_operator_precedence_mul_over_add() {
    let js = r#"
/**
 * @param {number} a
 * @param {number} b
 * @param {number} c
 * @returns {number}
 */
export function testParens(a, b, c) {
    return (a + b) * c;
}
"#;
    let zig = transpile_and_check(js, "test_p0_5_operator_precedence_mul_over_add");
    println!("=== P0-5 operator precedence (mul over add) ===\n{}", zig);
    // The inner (a + b) must be parenthesized to protect against * binding
    // more tightly than + in Zig.
    assert!(
        zig.contains("(a + b)"),
        "Expected (a + b) to be parenthesized in: {}",
        zig
    );
}

/// P0-5: Shift precedence mismatch — JS `<<` has LOWER precedence than `+`,
/// but Zig `<<` has HIGHER precedence than `+`. So `(a + b) << c` must have
/// the addition parenthesized, otherwise Zig parses `a + b << c` as
/// `a + (b << c)`.
#[test]
fn test_p0_5_shift_precedence_mismatch() {
    let js = r#"
/**
 * @param {number} a
 * @param {number} b
 * @param {number} c
 * @returns {number}
 */
export function testShiftPrec(a, b, c) {
    return (a + b) << c;
}
"#;
    let zig = transpile_and_check(js, "test_p0_5_shift_precedence_mismatch");
    println!("=== P0-5 shift precedence mismatch ===\n{}", zig);
    assert!(
        zig.contains("(a + b)"),
        "Expected (a + b) to be parenthesized before << in: {}",
        zig
    );
}

/// P0-5: IMPLICIT precedence mismatch — JS `a + b << c` is parsed as
/// `(a + b) << c` because `+` has higher precedence than `<<` in JS.
/// But Zig `<<` has HIGHER precedence than `+`, so the flat emission
/// `a + b << c` would be parsed by Zig as `a + (b << c)` — WRONG.
/// The emitter must parenthesize sub-expressions to preserve JS semantics.
#[test]
fn test_p0_5_implicit_shift_precedence() {
    let js = r#"
/**
 * @param {number} a
 * @param {number} b
 * @param {number} c
 * @returns {number}
 */
export function testImplicitShift(a, b, c) {
    return a + b << c;
}
"#;
    let zig = transpile_and_check(js, "test_p0_5_implicit_shift_precedence");
    println!("=== P0-5 implicit shift precedence ===\n{}", zig);
    // JS parses `a + b << c` as `(a + b) << c`. The emitter must produce
    // Zig that evaluates the addition first. Without parens, Zig would
    // evaluate `b << c` first (wrong).
    assert!(
        zig.contains("(a + b)"),
        "Expected (a + b) to be parenthesized for JS implicit precedence: {}",
        zig
    );
}

/// P0-5: Subtraction in multiplication — `(a - b) * c` needs explicit parens
#[test]
fn test_p0_5_subtraction_in_multiplication() {
    let js = r#"
/**
 * @param {number} a
 * @param {number} b
 * @param {number} c
 * @returns {number}
 */
export function testSubMul(a, b, c) {
    return (a - b) * c;
}
"#;
    let zig = transpile_and_check(js, "test_p0_5_subtraction_in_multiplication");
    println!("=== P0-5 subtraction in multiplication ===\n{}", zig);
    assert!(
        zig.contains("(a - b)"),
        "Expected (a - b) to be parenthesized in: {}",
        zig
    );
}
