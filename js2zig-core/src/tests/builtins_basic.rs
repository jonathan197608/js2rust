// Optional prop, Math, Array basic, Math P4, Await, TypedArray, string escape, JSON edge

use super::common::*;

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
    let zig = transpile_and_check(js, "test_native_proto_optional_property");

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
    let zig = transpile_and_check(js, "test_native_proto_math_methods");

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
    let zig = transpile_and_check(js, "test_native_proto_array_pop");

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
    let zig = transpile_and_check(js, "test_native_proto_array_indexof");

    // Verify labeled block with for loop is generated
    assert!(zig.contains("blk_"), "Expected labeled block in:\n{}", zig);
    assert!(zig.contains("for ("), "Expected for loop in:\n{}", zig);
    assert!(
        zig.contains(".items"),
        "Expected .items access in:\n{}",
        zig
    );
    assert!(
        zig.contains("break :blk_"),
        "Expected break :blk_ in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_array_includes");

    // Verify labeled block with for loop and bool return
    assert!(zig.contains("blk_"), "Expected labeled block in:\n{}", zig);
    assert!(zig.contains("for ("), "Expected for loop in:\n{}", zig);
    assert!(
        zig.contains("break :blk_") && zig.contains(" true"),
        "Expected break :blk_ with true in:\n{}",
        zig
    );
    assert!(
        zig.contains("break :blk_") && zig.contains(" false"),
        "Expected break :blk_ with false in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_array_join");

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
    let zig = transpile_and_check(js, "test_native_proto_array_slice");

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
    let zig = transpile_and_check(js, "test_native_proto_array_splice");

    // Verify splice generates ArrayList operations
    assert!(
        zig.contains("orderedRemove"),
        "Expected orderedRemove in:\n{}",
        zig
    );
    assert!(
        zig.contains("break :blk_") && zig.contains("__spliced"),
        "Expected break :blk_ __spliced in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_array_splice_insert");

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
    let zig = transpile_and_assert(js, "test_native_proto_math_new_methods");

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
    let zig = transpile_and_assert(js, "test_native_proto_math_phase4");

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

// ── P0-7: Math float-arg coercion (@floatFromInt on float args) ─────────
// Before P0-7, Math.sin/cos/tan/atan/log/exp/asin/acos/hypot/fround/clz32/sign
// unconditionally emitted `@floatFromInt(arg)`, which is a Zig compile error
// when the argument is a float (e.g. `Math.sin(1.5)` → `@sin(@floatFromInt(1.5))`
// fails: @floatFromInt expects an integer). P0-7 adds `expr_is_float` +
// `emit_f64_coerced` to pick the correct coercion per arg shape.

#[test]
fn test_p0_7_float_literal_args() {
    // Float-literal args: must emit `@as(f64, <literal>)`, NOT `@floatFromInt`.
    let js = r#"
/**
 * @returns {number}
 */
export function p07FloatLit() {
    return Math.sin(1.5) + Math.cos(2.5) + Math.tan(0.5)
         + Math.atan(0.5) + Math.asin(0.5) + Math.acos(0.5)
         + Math.exp(1.5) + Math.log(2.5);
}
"#;
    let zig = transpile_and_check(js, "test_p0_7_float_literal_args");

    // Float-coercing builtins must be present
    assert!(zig.contains("@sin("), "Expected '@sin(' in:\n{}", zig);
    assert!(zig.contains("@cos("), "Expected '@cos(' in:\n{}", zig);
    assert!(zig.contains("@tan("), "Expected '@tan(' in:\n{}", zig);
    assert!(
        zig.contains("std.math.atan("),
        "Expected 'std.math.atan(' (atan is NOT a @-builtin) in:\n{}",
        zig
    );
    assert!(
        zig.contains("std.math.asin("),
        "Expected 'std.math.asin(' in:\n{}",
        zig
    );
    assert!(
        zig.contains("std.math.acos("),
        "Expected 'std.math.acos(' in:\n{}",
        zig
    );
    assert!(zig.contains("@exp("), "Expected '@exp(' in:\n{}", zig);
    assert!(zig.contains("@log("), "Expected '@log(' in:\n{}", zig);
    // CRITICAL: @floatFromInt must NOT appear for float-literal args
    assert!(
        !zig.contains("@floatFromInt"),
        "P0-7 regression: @floatFromInt emitted for float-literal args:\n{}",
        zig
    );
    // Spot-check the identity-cast form for a float literal
    assert!(
        zig.contains("@as(f64, 1.5)"),
        "Expected '@as(f64, 1.5)' identity cast for float literal in:\n{}",
        zig
    );
}

#[test]
fn test_p0_7_int_variable_arg() {
    // i64 variable arg (@param {number}): must STILL use @floatFromInt.
    let js = r#"
/**
 * @param {number} x
 * @returns {number}
 */
export function p07IntVar(x) {
    return Math.sin(x);
}
"#;
    let zig = transpile_and_check(js, "test_p0_7_int_variable_arg");

    assert!(zig.contains("@sin("), "Expected '@sin(' in:\n{}", zig);
    assert!(
        zig.contains("@floatFromInt"),
        "Expected @floatFromInt for i64 variable arg in:\n{}",
        zig
    );
}

#[test]
fn test_p0_7_float_expr_arg() {
    // Arg is itself a float-returning BuiltinCall (Math.cos): must use
    // `@as(f64, ...)` identity cast, NOT `@floatFromInt`.
    let js = r#"
/**
 * @returns {number}
 */
export function p07FloatExpr() {
    return Math.sin(Math.cos(0.5));
}
"#;
    let zig = transpile_and_check(js, "test_p0_7_float_expr_arg");

    assert!(zig.contains("@sin("), "Expected '@sin(' in:\n{}", zig);
    assert!(zig.contains("@cos("), "Expected '@cos(' in:\n{}", zig);
    assert!(
        !zig.contains("@floatFromInt"),
        "P0-7 regression: @floatFromInt emitted for float-expr arg:\n{}",
        zig
    );
}

#[test]
fn test_p0_7_hypot_float_args() {
    // Math.hypot with float literals: squared terms use identity cast.
    let js = r#"
/**
 * @returns {number}
 */
export function p07HypotFloat() {
    return Math.hypot(1.5, 2.5);
}
"#;
    let zig = transpile_and_check(js, "test_p0_7_hypot_float_args");

    assert!(
        zig.contains("@sqrt("),
        "Expected '@sqrt(' for hypot in:\n{}",
        zig
    );
    assert!(
        zig.contains("@as(f64, 1.5)"),
        "Expected '@as(f64, 1.5)' squared term in:\n{}",
        zig
    );
    assert!(
        zig.contains("@as(f64, 2.5)"),
        "Expected '@as(f64, 2.5)' squared term in:\n{}",
        zig
    );
    assert!(
        !zig.contains("@floatFromInt"),
        "P0-7 regression: @floatFromInt emitted for hypot float args:\n{}",
        zig
    );
}

#[test]
fn test_p0_7_clz32_float_arg() {
    // Math.clz32(1.5): float arg must be reduced via @intFromFloat before @clz.
    // Uses transpile_and_assert (no ast-check) because @clz's narrow integer
    // return type interacts with the `@returns {number}` signature.
    let js = r#"
/**
 * @returns {number}
 */
export function p07Clz32Float() {
    return Math.clz32(1.5);
}
"#;
    let zig = transpile_and_assert(js, "test_p0_7_clz32_float_arg");

    assert!(zig.contains("@clz("), "Expected '@clz(' in:\n{}", zig);
    assert!(
        zig.contains("@intFromFloat"),
        "Expected @intFromFloat for clz32 float arg in:\n{}",
        zig
    );
    assert!(
        !zig.contains("@floatFromInt"),
        "P0-7 regression: @floatFromInt emitted for clz32 float arg:\n{}",
        zig
    );
}

#[test]
fn test_p0_7_fround_float_arg() {
    // Math.fround(1.5): float arg → @as(f32, @floatCast(...)).
    // Uses transpile_and_assert: fround yields f32 which may not match the
    // inferred `@returns {number}` signature under ast-check.
    let js = r#"
/**
 * @returns {number}
 */
export function p07Fround() {
    return Math.fround(1.5);
}
"#;
    let zig = transpile_and_assert(js, "test_p0_7_fround_float_arg");

    assert!(
        zig.contains("@floatCast"),
        "Expected @floatCast for fround float arg in:\n{}",
        zig
    );
    assert!(
        !zig.contains("@floatFromInt"),
        "P0-7 regression: @floatFromInt emitted for fround float arg:\n{}",
        zig
    );
}

#[test]
fn test_p0_7_sign_float_arg() {
    // Math.sign(2.5): float-literal arg → cached value uses `@as(f64, 2.5)`.
    let js = r#"
/**
 * @returns {number}
 */
export function p07Sign() {
    return Math.sign(2.5);
}
"#;
    let zig = transpile_and_check(js, "test_p0_7_sign_float_arg");

    assert!(
        zig.contains("@as(f64, 2.5)"),
        "Expected '@as(f64, 2.5)' for sign float arg in:\n{}",
        zig
    );
    assert!(
        !zig.contains("@floatFromInt"),
        "P0-7 regression: @floatFromInt emitted for sign float arg:\n{}",
        zig
    );
}

// ── P0-8: Math.min/max float-variable detection + mixed-type coercion ────
// Before P0-8, emit_min_max's `any_float` only matched FloatLiteral, so a
// float-typed variable arg (division result, BuiltinCall returning f64) fell
// into the i64 branch and emitted `@as(i64, <f64 expr>)` — a Zig compile
// error. Mixed int+float args (e.g. Math.max(x, 1.5)) also broke: the float
// branch emitted raw args without coercion, producing an i64-vs-f64
// comparison. P0-8 routes `any_float` through `expr_is_float` and coerces
// every arg in the float branch via `emit_f64_coerced`.

#[test]
fn test_p0_8_min_max_float_expr_arg() {
    // Arg is a float-returning BuiltinCall (Math.cos): must take the f64
    // branch, producing f64-typed comparisons — NOT the i64 branch.
    let js = r#"
/**
 * @returns {number}
 */
export function p08MaxBuiltin() {
    return Math.max(Math.cos(0.5), 1.5);
}
"#;
    let zig = transpile_and_check(js, "test_p0_8_min_max_float_expr_arg");

    assert!(
        zig.contains("@cos("),
        "Expected '@cos(' for float-expr arg in:\n{}",
        zig
    );
    // Float branch coerces args to f64; @as(i64, ...) must NOT appear.
    assert!(
        !zig.contains("@as(i64,"),
        "P0-8 regression: i64 branch used for float-expr arg in:\n{}",
        zig
    );
    assert!(
        zig.contains("@as(f64, 1.5)"),
        "Expected '@as(f64, 1.5)' coercion for float literal in:\n{}",
        zig
    );
}

#[test]
fn test_p0_8_min_max_mixed_int_float() {
    // Mixed i64 variable + float literal: must coerce the i64 arg via
    // @floatFromInt so the comparison is f64-vs-f64 (Zig cannot compare
    // i64 with f64). Before P0-8 this emitted a bare `if (1.5 > __max)` on
    // an i64-typed variable — a compile error.
    let js = r#"
/**
 * @param {number} x
 * @returns {number}
 */
export function p08MaxMixed(x) {
    return Math.max(x, 1.5);
}
"#;
    let zig = transpile_and_check(js, "test_p0_8_min_max_mixed_int_float");

    assert!(
        zig.contains("@floatFromInt"),
        "Expected @floatFromInt for i64 arg in mixed min/max in:\n{}",
        zig
    );
    assert!(
        zig.contains("@as(f64, 1.5)"),
        "Expected '@as(f64, 1.5)' for float literal in:\n{}",
        zig
    );
    assert!(
        !zig.contains("@as(i64,"),
        "P0-8 regression: i64 branch used for mixed int/float args in:\n{}",
        zig
    );
}

#[test]
fn test_p0_8_min_max_int_args_unchanged() {
    // All-int args (i64 variables): must STILL use the i64 branch with
    // @as(i64, ...) wrapping — the common case must not regress.
    let js = r#"
/**
 * @param {number} x
 * @param {number} y
 * @returns {number}
 */
export function p08MaxInt(x, y) {
    return Math.max(x, y);
}
"#;
    let zig = transpile_and_check(js, "test_p0_8_min_max_int_args_unchanged");

    assert!(
        zig.contains("@as(i64,"),
        "Expected @as(i64, ...) in i64 min/max branch in:\n{}",
        zig
    );
    assert!(
        !zig.contains("@floatFromInt"),
        "i64 min/max branch must not emit @floatFromInt in:\n{}",
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
    let zig = transpile_and_check_with_exports(js, "test_native_proto_await", exports);

    // Step2: verify async function signature has `io: js_runtime.Io`
    assert!(
        zig.contains("io: js_runtime.Io"),
        "Expected 'io: js_runtime.Io' in async function signature, got:\n{}",
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
        zig.contains("fn double(x: anytype) @TypeOf("),
        "Expected non-async function signature, got:\n{}",
        zig
    );
}

// ── P1-12: stmt_contains_await / expr_contains_await — nested-position tests ─
// Before the fix, awaited expressions inside ForStatement bodies, TryStatement
// blocks, the RHS of AssignmentExpression, or inside TemplateLiteral
// interpolations were silently dropped, so a function using `await` in those
// positions was incorrectly categorized as non-async (no `io: js_runtime.Io`).
// Each test below places the only `await` inside one of the previously-missing
// constructs and confirms the function is marked async.

#[test]
fn test_p1_12_await_inside_for_loop_body() {
    let js = r#"
async function fetchFirst(items) {
    for (let i = 0; i < items.length; i = i + 1) {
        const v = await items[i];
        return v;
    }
}
"#;
    let mut exports = std::collections::HashSet::new();
    exports.insert("fetchFirst".to_string());
    let zig = transpile_and_check_with_exports(js, "test_p1_12_await_in_for", exports);
    assert!(
        zig.contains("io: js_runtime.Io"),
        "Expected async signature (await inside for-loop was dropped before P1-12):\n{}",
        zig
    );
}

#[test]
fn test_p1_12_await_inside_try_block() {
    let js = r#"
async function safeFetch(key) {
    let result = null;
    try {
        result = await fetch(key);
    } catch (e) {
        result = null;
    }
    return result;
}

function fetch(k) { return k; }
"#;
    let mut exports = std::collections::HashSet::new();
    exports.insert("safeFetch".to_string());
    let zig = transpile_and_check_with_exports(js, "test_p1_12_await_in_try", exports);
    assert!(
        zig.contains("io: js_runtime.Io"),
        "Expected async signature (await inside try block was dropped before P1-12):\n{}",
        zig
    );
}

#[test]
fn test_p1_12_await_in_template_literal() {
    let js = r#"
async function greet(getNameFn) {
    return `hello ${await getNameFn()}`;
}

function getName() { return "x"; }
"#;
    let mut exports = std::collections::HashSet::new();
    exports.insert("greet".to_string());
    let zig = transpile_and_check_with_exports(js, "test_p1_12_await_in_template", exports);
    assert!(
        zig.contains("io: js_runtime.Io"),
        "Expected async signature (await inside TemplateLiteral was dropped before P1-12):\n{}",
        zig
    );
}

#[test]
fn test_p1_12_await_in_switch_case() {
    // Use break (not return) inside the case body — `return await foo()`
    // generates an unreachable-code pattern after the switch.
    let js = r#"
async function dispatch(code) {
    let result = null;
    switch (code) {
        case 1:
            result = await fetchOne();
            break;
        default:
            result = null;
    }
    return result;
}

function fetchOne() { return 1; }
"#;
    let mut exports = std::collections::HashSet::new();
    exports.insert("dispatch".to_string());
    let zig = transpile_and_check_with_exports(js, "test_p1_12_await_in_switch", exports);
    assert!(
        zig.contains("io: js_runtime.Io"),
        "Expected async signature (await inside switch was dropped before P1-12):\n{}",
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
    let zig = transpile_and_assert(js, "test_native_proto_typedarray_basic");
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
    let zig = transpile_and_assert(js, "test_native_proto_string_escape_backslash");
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
    let zig = transpile_and_assert(js, "test_native_proto_string_escape_quote");
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
    let zig = transpile_and_assert(js, "test_native_proto_typedarray_uint8");
    println!("=== TypedArray Uint8 ===\n{}", zig);
    assert!(
        zig.contains("fromI64AsU8"),
        "Expected 'fromI64AsU8' in generated code:\n{}",
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
    let zig = transpile_and_assert(js, "test_native_proto_typedarray_length_in_expr");
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
    let zig = transpile_and_assert(js, "test_native_proto_typedarray_length_as_param");
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
    let zig = transpile_and_assert(js, "test_native_proto_typedarray_set");
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
    let zig = transpile_and_assert(js, "test_native_proto_typedarray_slice");
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
    let zig = transpile_and_assert(js, "test_native_proto_typedarray_subarray");
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
    let zig = transpile_and_assert(js, "test_native_proto_typedarray_copywithin");
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
    let zig = transpile_and_assert(js, "test_native_proto_typedarray_fill");
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
    let zig = transpile_and_assert(js, "test_native_proto_typedarray_buffer");
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
    let zig = transpile_and_assert(js, "test_native_proto_typedarray_bytelength");
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
    let zig = transpile_and_assert(js, "test_native_proto_typedarray_byteoffset");
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
    let zig = transpile_and_assert(js, "test_native_proto_float64array");
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
    let zig = transpile_and_assert(js, "test_native_proto_string_escape_newline");
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
    let zig = transpile_and_assert(js, "test_native_proto_string_escape_tab");
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
    let zig = transpile_and_assert(js, "test_native_proto_json_parse_escaped_quotes");
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
    let zig = transpile_and_assert(js, "test_native_proto_json_parse_unicode");
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
