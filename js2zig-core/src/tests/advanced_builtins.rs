// Number, Map, String P6, Set, Object, URI, RegExp, Symbol, matchAll, mixed decl

use super::common::*;

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
    let zig = transpile_and_check(js, "test_native_proto_number_constants");

    // Verify all 8 constants are generated as Zig equivalents
    assert!(
        zig.contains("std.math.floatMax(f64)"),
        "Expected 'std.math.floatMax(f64)' in:\n{}",
        zig
    );
}

// ── BUG-02: arguments object tests ──

#[test]
fn test_arguments_length() {
    let js = r#"
/** @returns {i64} */
export function argLen(a, b, c) {
    return arguments.length;
}
"#;
    let zig = transpile_and_check(js, "test_arguments_length");
    // __arguments is [&]JsAny, .length should be .len (slice length)
    assert!(
        zig.contains("__arguments.len"),
        "Expected '__arguments.len' for arguments.length, got:\n{}",
        zig
    );
    assert!(
        !zig.contains("utf16Len"),
        "Should NOT contain utf16Len for arguments.length, got:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_number_issafeinteger");
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
    let zig = transpile_and_check(js, "test_native_proto_number_tofixed");
    assert!(
        zig.contains("js_number.toFixed(js_allocator.allocator(), pi"),
        "Expected 'js_number.toFixed(js_allocator.allocator(), pi' in:\n{}",
        zig
    );
}

// ── Test: Number.prototype.toString() [radix] (R8-NumberToString) ──
// Previously every `.toString()` call was silently mis-routed to
// `js_date.toString`, producing wrong output for literals and a Zig
// compile error for variable receivers. These four tests verify the
// new routing through `js_number.toString` for both literal receivers
// (handled in detect_builtin_call) and F64/I64 variable receivers
// (handled by the lowerer's "fix string-variable methods" block).

#[test]
fn test_native_proto_number_tostring_literal_default_radix() {
    let js = r#"
/**
 * @returns {string}
 */
export function defaultRadixLiteral() {
return (42).toString();
}
"#;
    let zig = transpile_and_check(
        js,
        "test_native_proto_number_tostring_literal_default_radix",
    );
    // No-radix literal: emitter must supply the ECMA-262 default radix 10
    // since the Zig runtime signature `toString(alloc, val, radix: i64)`
    // has no Zig default-parameter support.
    assert!(
        zig.contains("js_number.toString(js_allocator.allocator(),"),
        "Expected 'js_number.toString(js_allocator.allocator(),' in:\n{}",
        zig
    );
    // The numeric literal value 42 must be inlined and the default radix
    // 10 must be appended when the JS call omits it.
    assert!(
        zig.contains(", 10)"),
        "Expected ', 10)' (default radix appended) in:\n{}",
        zig
    );
    // Should NOT route to js_date.toString for a numeric literal receiver.
    assert!(
        !zig.contains("js_date.toString"),
        "Must NOT route (42).toString() to js_date.toString:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_number_tostring_literal_explicit_radix() {
    let js = r#"
/**
 * @returns {string}
 */
export function hexLiteral() {
return (255).toString(16);
}
"#;
    let zig = transpile_and_check(
        js,
        "test_native_proto_number_tostring_literal_explicit_radix",
    );
    // Explicit radix: the user-supplied 16 must be emitted.
    assert!(
        zig.contains("js_number.toString(js_allocator.allocator(),"),
        "Expected 'js_number.toString(js_allocator.allocator(),' in:\n{}",
        zig
    );
    assert!(
        zig.contains(", 16)"),
        "Expected ', 16)' (explicit radix) in:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_number_tostring_var_f64_radix() {
    let js = r#"
/**
 * @param {number} n
 * @returns {string}
 */
export function varF64Radix(n) {
return n.toString(2);
}
"#;
    let zig = transpile_and_check(js, "test_native_proto_number_tostring_var_f64_radix");
    // F64 variable receiver: detect_builtin_call routes `.toString()` to
    // DateToString (no type info at AST layer); the lowerer's "fix
    // string-variable methods" block rewrites the module to JsNumber.
    assert!(
        zig.contains("js_number.toString(js_allocator.allocator(), n, 2)"),
        "Expected 'js_number.toString(js_allocator.allocator(), n, 2)' in:\n{}",
        zig
    );
    assert!(
        !zig.contains("js_date.toString"),
        "Must NOT route F64-var .toString() to js_date.toString:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_number_tostring_var_i64_no_radix() {
    let js = r#"
/**
 * @param {i64} n
 * @returns {string}
 */
export function varI64Default(n) {
return n.toString();
}
"#;
    let zig = transpile_and_check(js, "test_native_proto_number_tostring_var_i64_no_radix");
    // I64 variable receiver, no-radix: lowerer rewrites DateToString→JsNumber,
    // emitter adds the ECMA-262 default radix 10.
    assert!(
        zig.contains("js_number.toString(js_allocator.allocator(), n, 10)"),
        "Expected 'js_number.toString(js_allocator.allocator(), n, 10)' in:\n{}",
        zig
    );
}

// ── BigInt.prototype.toString(radix) — R8-P1-4 ──
// Previously BigInt.toString hard-coded base 10 and silently dropped any
// radix argument. Now radix is propagated to the runtime, and BigInt
// literal receivers route to BigIntToString instead of DateToString.

#[test]
fn test_native_proto_bigint_tostring_literal_explicit_radix() {
    let js = r#"
/**
 * @returns {string}
 */
export function hexLiteral() {
return 255n.toString(16);
}
"#;
    let zig = transpile_and_check(
        js,
        "test_native_proto_bigint_tostring_literal_explicit_radix",
    );
    // BigInt literal receiver: extract_callee_object_name_static inlines
    // a JsBigInt init expression as the receiver.
    assert!(
        zig.contains("js_bigint.JsBigInt.init(js_allocator.allocator(), \"255\")"),
        "Expected BigInt init expression as receiver in:\n{}",
        zig
    );
    // Explicit radix 16 must be emitted.
    assert!(
        zig.contains(".toString(js_allocator.allocator(), 16)"),
        "Expected '.toString(js_allocator.allocator(), 16)' in:\n{}",
        zig
    );
    // Should NOT route to js_date.toString for a BigInt literal receiver.
    assert!(
        !zig.contains("js_date.toString"),
        "Must NOT route 255n.toString() to js_date.toString:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_bigint_tostring_literal_default_radix() {
    let js = r#"
/**
 * @returns {string}
 */
export function defaultRadixLiteral() {
return 255n.toString();
}
"#;
    let zig = transpile_and_check(
        js,
        "test_native_proto_bigint_tostring_literal_default_radix",
    );
    // No-radix literal: emitter must supply the ECMA-262 default radix 10.
    assert!(
        zig.contains(".toString(js_allocator.allocator(), 10)"),
        "Expected '.toString(js_allocator.allocator(), 10)' (default radix) in:\n{}",
        zig
    );
    assert!(
        !zig.contains("js_date.toString"),
        "Must NOT route 255n.toString() to js_date.toString:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_bigint_tostring_var_explicit_radix() {
    let js = r#"
/**
 * @returns {string}
 */
export function varRadix() {
let b = 255n;
return b.toString(16);
}
"#;
    let zig = transpile_and_check(js, "test_native_proto_bigint_tostring_var_explicit_radix");
    // BigInt variable receiver: the lowerer's BigInt variable interception
    // block rewrites to BuiltinModule::JsBigInt. The user radix 16 must be
    // emitted.
    assert!(
        zig.contains("b.toString(js_allocator.allocator(), 16)"),
        "Expected 'b.toString(js_allocator.allocator(), 16)' in:\n{}",
        zig
    );
    assert!(
        !zig.contains("js_date.toString"),
        "Must NOT route BigInt-var .toString() to js_date.toString:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_bigint_tostring_var_default_radix() {
    let js = r#"
/**
 * @returns {string}
 */
export function varDefault() {
let b = 255n;
return b.toString();
}
"#;
    let zig = transpile_and_check(js, "test_native_proto_bigint_tostring_var_default_radix");
    // No-radix BigInt variable: emitter appends the default radix 10.
    assert!(
        zig.contains("b.toString(js_allocator.allocator(), 10)"),
        "Expected 'b.toString(js_allocator.allocator(), 10)' in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_map_foreach");

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
    // Key binding: only emitted when the callback's key param is used in the body.
    // In this test, only `val` is used, so key binding is correctly skipped.
    // (If the body used `key`, we'd see `const key = entry.key_ptr.*;`)
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
    let zig = transpile_and_check(js, "test_p6_string_starts_with");
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
    let zig = transpile_and_check(js, "test_p6_string_ends_with");
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
    let zig = transpile_and_check(js, "test_p6_string_includes");
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
    let zig = transpile_and_check(js, "test_p6_string_repeat");
    assert!(
        zig.contains("js_string.repeat(js_allocator.allocator()"),
        "Expected 'js_string.repeat(js_allocator.allocator()' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p6_string_substring");
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
    let zig = transpile_and_check(js, "test_p6_string_slice");
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
    let zig = transpile_and_check(js, "test_p6_string_concat");
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
    let zig = transpile_and_check(js, "test_p6_string_normalize");
    assert!(
        zig.contains("js_string_icu.normalize(js_allocator.allocator()"),
        "Expected 'js_string_icu.normalize(js_allocator.allocator()' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p6_string_to_upper_case");
    assert!(
        zig.contains("js_string.toUpper(js_allocator.allocator()"),
        "Expected 'js_string.toUpper(js_allocator.allocator()' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p6_string_to_lower_case");
    assert!(
        zig.contains("js_string.toLower(js_allocator.allocator()"),
        "Expected 'js_string.toLower(js_allocator.allocator()' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p6_string_split");
    assert!(
        zig.contains("js_string.split(js_allocator.allocator()"),
        "Expected 'js_string.split(js_allocator.allocator()' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p6_string_char_at");
    assert!(
        zig.contains("js_string.charAt(js_allocator.allocator()"),
        "Expected 'js_string.charAt(js_allocator.allocator()' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p6_string_index_of");
    assert!(
        zig.contains("js_string.indexOf("),
        "Expected 'js_string.indexOf(' in:\n{}",
        zig
    );
    // R8-P1-19: missing fromIndex defaults to 0 (third positional arg).
    assert!(
        zig.contains("js_string.indexOf(hello, \"lo\", 0)")
            || zig.contains("js_string.indexOf(hello__slice__alias, \"lo\", 0)")
            || zig.contains(", \"lo\", 0)"),
        "Expected 'indexOf' to emit default fromIndex=0 (third arg) in:\n{}",
        zig
    );
}

// Test: String.indexOf() with explicit fromIndex (R8-P1-19)
#[test]
fn test_p19_string_index_of_from_index() {
    let js = r#"
/**
 * @returns {i64}
 */
export function findIndexFrom() {
return "hello hello".indexOf("o", 1);
}
"#;
    let zig = transpile_and_check(js, "test_p19_string_index_of_from_index");
    assert!(
        zig.contains("js_string.indexOf("),
        "Expected 'js_string.indexOf(' in:\n{}",
        zig
    );
    // The explicit fromIndex argument should appear as the third parameter.
    assert!(
        zig.contains(", 1)"),
        "Expected explicit fromIndex '1' as the third arg in:\n{}",
        zig
    );
    // Ensure no default-0 override is appended when JS provides fromIndex.
    assert!(
        !zig.contains(", 1, 0)"),
        "Should NOT append a default-0 after explicit fromIndex in:\n{}",
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
    let zig = transpile_and_check(js, "test_p6_string_pad_start");
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
    let zig = transpile_and_check(js, "test_p6_string_pad_end");
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
    let zig = transpile_and_check(js, "test_p6_string_replace");
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
    let zig = transpile_and_check(js, "test_p6_string_replace_all");
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
    let zig = transpile_and_check(js, "test_p6_string_char_code_at");
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
    let zig = transpile_and_check(js, "test_p6_string_code_point_at");
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
    let zig = transpile_and_check(js, "test_p6_string_to_locale_upper_case");
    assert!(
        zig.contains("js_string_icu.toLocaleUpper("),
        "Expected 'js_string_icu.toLocaleUpper(' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p6_string_to_locale_lower_case");
    assert!(
        zig.contains("js_string_icu.toLocaleLower("),
        "Expected 'js_string_icu.toLocaleLower(' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p6_string_locale_compare");
    assert!(
        zig.contains("js_string_icu.localeCompare("),
        "Expected 'js_string_icu.localeCompare(' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p6_string_from_char_code");
    assert!(
        zig.contains("js_string.fromCharCode(js_allocator.allocator(), &[_]i64{")
            && zig.contains("65, 66, 67"),
        "Expected fromCharCode to prepend js_allocator and pack args into i64 slice (R8-P1-17): {}",
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
    let zig = transpile_and_check(js, "test_p6_string_from_code_point");
    assert!(
        zig.contains("js_string.fromCodePoint(js_allocator.allocator(), &[_]i64{")
            && zig.contains("128512"),
        "Expected fromCodePoint to prepend js_allocator and pack args into i64 slice (R8-P1-18): {}",
        zig
    );
    // R8-P1-18: invalid code points must route `error.RangeError` through a
    // catch switch (rather than swallow it as a generic OOM panic).
    assert!(
        zig.contains("catch |err| switch (err)"),
        "Expected fromCodePoint emission to handle error.RangeError via catch switch (R8-P1-18): {}",
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
    let zig = transpile_and_check(js, "test_p7_set_add_has");
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
    let zig = transpile_and_check(js, "test_p7_set_foreach");
    // Set.forEach now uses while-iterator (same as for-of Set),
    // since JsCollection(void) doesn't have an .items field.
    assert!(
        zig.contains("var iter = s.inner.iterator();"),
        "Expected 'var iter = s.inner.iterator();' in:\n{}",
        zig
    );
    assert!(
        zig.contains("while (iter.next()) |entry|"),
        "Expected 'while (iter.next()) |entry|' in:\n{}",
        zig
    );
    // Set stores values as keys (value type is void), so use key_ptr.*
    assert!(
        zig.contains("entry.key_ptr.*"),
        "Expected 'entry.key_ptr.*' for Set values in:\n{}",
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
    let zig = transpile_and_check(js, "test_p7_set_iterators");
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
    let zig = transpile_and_check(js, "test_p7_set_delete_clear");
    assert!(
        zig.contains(".delete(js_allocator.allocator(),"),
        "Expected '.delete(js_allocator.allocator(),' in:\n{}",
        zig
    );
    assert!(
        zig.contains(".clear(js_allocator.allocator())"),
        "Expected '.clear(js_allocator.allocator())' in:\n{}",
        zig
    );
}

// Phase 7: Object defineProperties / getOwnPropertyDescriptor / setPrototypeOf

#[test]
fn test_p7_object_define_properties() {
    let js = r#"
export function defineProps(target, props) {
Object.defineProperties(target, props);
}
"#;
    let zig = transpile_and_check(js, "test_p7_object_define_properties");
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
    let zig = transpile_and_check(js, "test_p7_object_get_own_property_descriptor");
    assert!(
        zig.contains("js_object.getOwnPropertyDescriptor(js_allocator.allocator(), "),
        "Expected 'js_object.getOwnPropertyDescriptor(js_allocator.allocator(), ' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p7_object_set_prototype_of");
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
    let zig = transpile_and_check(js, "test_p8_object_is_sealed_frozen_extensible");
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
    // /pattern/.test(str) → host_regex.regex_test("pattern", str)
    let js = r#"
export function hasDigit(s) {
return /\d/.test(s);
}
"#;
    let result = parse_and_transpile(js, None).unwrap();
    let zig = result.zig_code;
    assert!(
        zig.contains(r#"host_regex.regex_test("\\d", s)"#),
        "Expected 'host_regex.regex_test(\"\\d\", s)' in:\n{}",
        zig
    );
}

#[test]
fn test_p8_string_search() {
    // str.search(/pattern/) → host_regex.regex_search("pattern", str)
    let js = r#"
export function findDigit(s) {
return s.search(/\d+/);
}
"#;
    let result = parse_and_transpile(js, None).unwrap();
    let zig = result.zig_code;
    assert!(
        zig.contains(r#"host_regex.regex_search("\\d+", s)"#),
        "Expected 'host_regex.regex_search(\"\\d+\", s)' in:\n{}",
        zig
    );
}

#[test]
fn test_p8_string_match_compile_error() {
    // str.match(/pattern/) → js_string_regex.matchString(alloc, str, "pattern")
    let js = r#"
export function getMatch(s) {
return s.match(/hello/);
}
"#;
    let result = parse_and_transpile(js, None).unwrap();
    let zig = result.zig_code;
    assert!(
        zig.contains(r#"js_string_regex.matchString(js_allocator.allocator(),"#),
        "Expected 'js_string_regex.matchString(js_allocator.allocator(),' for String.match() in:\n{}",
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
    // new RegExp(pattern) → js_regexp.JsRegExp.init(alloc, pattern) catch @panic(...)
    let js = r#"
export function makePattern(s) {
const r = new RegExp("\\d+");
return r.test(s);
}
"#;
    let result = parse_and_transpile(js, None).unwrap();
    let zig = result.zig_code;
    assert!(
        zig.contains("js_regexp.JsRegExp.init(js_allocator.allocator(),"),
        "Expected 'js_regexp.JsRegExp.init(...)' for new RegExp in:\n{}",
        zig
    );
    assert!(
        zig.contains("catch @panic(\"OOM: RegExp init\")"),
        "Expected 'catch @panic(\"OOM: RegExp init\")' for new RegExp in:\n{}",
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
    // str.match(regexpVar) → js_string_regex.matchString(alloc, str, regexpVar.pattern)
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
    // str.search(regexpVar) → host_regex.regex_search(regexpVar.pattern, str)
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
        zig.contains("js_regexp.execLiteral(js_allocator.allocator(),"),
        "Expected 'js_regexp.execLiteral(js_allocator.allocator(),' in:\n{}",
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
        zig.contains(".exec(js_allocator.allocator(),"),
        "Expected '.exec(js_allocator.allocator(),' for regexpVar.exec() in:\n{}",
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
    let zig = transpile_and_check(js, "test_p7_encode_uri");
    assert!(
        zig.contains("js_uri.encodeURI(js_allocator.allocator(),"),
        "Expected 'js_uri.encodeURI(js_allocator.allocator(),' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p7_decode_uri");
    assert!(
        zig.contains("js_uri.decodeURI(js_allocator.allocator(),"),
        "Expected 'js_uri.decodeURI(js_allocator.allocator(),' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p7_encode_uri_component");
    assert!(
        zig.contains("js_uri.encodeURIComponent(js_allocator.allocator(),"),
        "Expected 'js_uri.encodeURIComponent(js_allocator.allocator(),' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p7_decode_uri_component");
    assert!(
        zig.contains("js_uri.decodeURIComponent(js_allocator.allocator(),"),
        "Expected 'js_uri.decodeURIComponent(js_allocator.allocator(),' in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_map_get_eq_cmp");
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
fn test_in_operator_map() {
    // `key in map` → map.has(JsAny.from(key)), NOT map.contains(key)
    let js = r#"
export function hasMapKey(k) {
    const m = new Map();
    return k in m;
}
"#;
    let zig = transpile_and_check(js, "test_in_operator_map");
    assert!(
        zig.contains(".has(JsAny.from("),
        "Expected '.has(JsAny.from(...))' for 'in' operator on Map, got:\n{}",
        zig
    );
    assert!(
        !zig.contains(".contains("),
        "Should NOT contain '.contains()' for Map 'in' operator, got:\n{}",
        zig
    );
}

#[test]
fn test_in_operator_set() {
    // `key in set` → set.has(JsAny.from(key)), NOT set.contains(key)
    let js = r#"
export function hasSetKey(k) {
    const s = new Set();
    return k in s;
}
"#;
    let zig = transpile_and_check(js, "test_in_operator_set");
    assert!(
        zig.contains(".has(JsAny.from("),
        "Expected '.has(JsAny.from(...))' for 'in' operator on Set, got:\n{}",
        zig
    );
    assert!(
        !zig.contains(".contains("),
        "Should NOT contain '.contains()' for Set 'in' operator, got:\n{}",
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
    let _zig = transpile_and_check(js, "p8_string_match_ast_check");
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
    let _zig = transpile_and_check(js, "p8_string_match_regexp_var_ast_check");
}

#[test]
fn test_p8_string_match_global_ast_check() {
    // Verify that String.match(/pattern/g) generates code that calls matchStringGlobal.
    let js = r#"
export function getMatch(s) {
return s.match(/world/g);
}
"#;
    let zig = transpile_and_check(js, "p8_string_match_global_ast_check");
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
    let _zig = transpile_and_check(js, "p8_string_match_capture_groups_ast_check");
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
    let _zig = transpile_and_check(js, "p8_string_match_empty_pattern_ast_check");
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
    let _zig = transpile_and_check(js, "p8_string_match_empty_string_ast_check");
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
    let zig = transpile_and_check(js, "test_native_proto_symbol_basic");
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
    let zig = transpile_and_check(js, "test_native_proto_symbol_for_keyfor");
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
    let zig = transpile_and_check(js, "test_native_proto_symbol_description");
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
    // R8-NumberToString note: the @param {Symbol} annotation is required
    // because the new F64/I64-variable interception in lower/expr/call.rs
    // (Step 1 "fix string-variable methods" block) routes `.toString()` on
    // i64-typed receivers to js_number.toString. Without the annotation
    // the default inference makes sym an i64 and the test would no longer
    // exercise Symbol's own toString, but rather the Number path. The
    // annotation routes sym to JsSymbol so the lowerer falls through to
    // (JsDate, "toString"), which emit_date_builtin renders as the
    // generic `sym.toString(js_allocator.allocator())` instance method
    // call.
    let js = r#"
/**
 * @param {Symbol} sym
 * @returns {string}
 */
export function symbolToString(sym) {
return sym.toString();
}
"#;
    let zig = transpile_and_check(js, "test_native_proto_symbol_toString");
    println!("=== Symbol.toString() ===\n{}", zig);
    // Should generate sym.toString(js_allocator.allocator())
    assert!(
        zig.contains("sym.toString(js_allocator.allocator())"),
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
    let zig = transpile_and_check(js, "test_native_proto_symbol_equality");
    println!("=== Symbol equality ===\n{}", zig);
    // Should generate === comparison for symbols
    assert!(
        zig.contains("s1") && zig.contains("s2"),
        "Expected s1 and s2 in code: {}",
        zig
    );
    // Symbol equality uses .eql() since JsSymbol is a struct with slice fields
    // (Zig doesn't support == on structs with slice fields).
    assert!(
        zig.contains(".eql("),
        "Expected .eql() for Symbol equality: {}",
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
    let zig = transpile_and_check(js, "test_native_proto_symbol_type_inference");
    println!("=== Symbol type inference ===\n{}", zig);
    // sym should be typed as JsSymbol
    assert!(
        zig.contains("sym: JsSymbol"),
        "Expected 'sym: JsSymbol' parameter type: {}",
        zig
    );
}

#[test]
fn test_native_proto_symbol_well_known_iterator() {
    // Symbol.iterator → js_symbol.symbolIterator()
    let js = r#"
/**
 * @returns {Symbol}
 */
export function getIteratorSymbol() {
return Symbol.iterator;
}
"#;
    let zig = transpile_and_check(js, "test_native_proto_symbol_well_known_iterator");
    println!("=== Symbol.iterator ===\n{}", zig);
    assert!(
        zig.contains("js_symbol.symbolIterator()"),
        "Expected js_symbol.symbolIterator() in:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_symbol_well_known_async_iterator() {
    // Symbol.asyncIterator → js_symbol.symbolAsyncIterator()
    let js = r#"
/**
 * @returns {Symbol}
 */
export function getAsyncIteratorSymbol() {
return Symbol.asyncIterator;
}
"#;
    let zig = transpile_and_check(js, "test_native_proto_symbol_well_known_async_iterator");
    println!("=== Symbol.asyncIterator ===\n{}", zig);
    assert!(
        zig.contains("js_symbol.symbolAsyncIterator()"),
        "Expected js_symbol.symbolAsyncIterator() in:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_symbol_well_known_multiple() {
    // Multiple well-known symbols in one function
    let js = r#"
/**
 * @returns {boolean}
 */
export function checkWellKnownSymbols() {
const iter = Symbol.iterator;
const match = Symbol.match;
const tag = Symbol.toStringTag;
return iter.id == match.id;
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_symbol_well_known_multiple");
    println!("=== Multiple well-known symbols ===\n{}", zig);
    assert!(
        zig.contains("js_symbol.symbolIterator()"),
        "Expected symbolIterator in:\n{}",
        zig
    );
    assert!(
        zig.contains("js_symbol.symbolMatch()"),
        "Expected symbolMatch in:\n{}",
        zig
    );
    assert!(
        zig.contains("js_symbol.symbolToStringTag()"),
        "Expected symbolToStringTag in:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_symbol_well_known_to_string_tag() {
    // Symbol.toStringTag used as property key
    let js = r#"
/**
 * @returns {Symbol}
 */
export function getToStringTag() {
return Symbol.toStringTag;
}
"#;
    let zig = transpile_and_check(js, "test_native_proto_symbol_well_known_to_string_tag");
    println!("=== Symbol.toStringTag ===\n{}", zig);
    assert!(
        zig.contains("js_symbol.symbolToStringTag()"),
        "Expected js_symbol.symbolToStringTag() in:\n{}",
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
    let zig = transpile_and_check(js, "p8_string_match_global_empty_match_ast_check");
    assert!(
        zig.contains("matchStringGlobal"),
        "Expected 'matchStringGlobal' for /g flag in:\n{}",
        zig
    );
}

#[test]
fn test_p3_string_match_all_ast_check() {
    // str.matchAll(/pattern/g) → js_string_regex.matchAllString(alloc, str, "pattern")
    let js = r#"
export function matchAllTest(s) {
return s.matchAll(/(\d)(\d)/g);
}
"#;
    let zig = transpile_and_check(js, "p3_string_match_all_ast_check");
    println!("=== String.matchAll ===\n{}", zig);
    assert!(
        zig.contains("matchAllString"),
        "Expected 'matchAllString' in:\n{}",
        zig
    );
}

#[test]
fn test_p3_string_match_all_regexp_var_ast_check() {
    // str.matchAll(regexpVar) → js_string_regex.matchAllString(alloc, str, regexpVar.pattern)
    let js = r#"
export function matchAllVarTest(s) {
const re = new RegExp("(\\d)(\\d)", "g");
return s.matchAll(re);
}
"#;
    let zig = transpile_and_check(js, "p3_string_match_all_regexp_var_ast_check");
    println!("=== String.matchAll (regexp var) ===\n{}", zig);
    assert!(
        zig.contains("matchAllString"),
        "Expected 'matchAllString' in:\n{}",
        zig
    );
    assert!(
        zig.contains(".pattern"),
        "Expected '.pattern' for regexp variable in:\n{}",
        zig
    );
}

// ── #768: 声明+表达式混合 — 验证不产生未使用变量/值警告 ──

// ── BUG-06: ArrayList type tracking tests ──

#[test]
fn test_bug08_string_padstart_literal() {
    // BUG-08: const s = "5"; s.padStart(3, "0") should NOT emit .deinit() on result
    let js = r#"
/** @returns {i64} */
export function testStringPadStart() {
    const s = "5";
    const padded = s.padStart(3, "0");
    if (padded === "005") {
        return 1;
    }
    return 0;
}
"#;
    let zig = transpile_and_check(js, "test_bug08_string_padstart_literal");
    // padded is a dynamically allocated string (padStart allocates) — should NOT have deinit
    // because ZigType::Str variables don't own resources that need cleanup
    assert!(
        !zig.contains("padded.deinit"),
        "String result of padStart should NOT have .deinit(), got:\n{}",
        zig
    );
    assert!(
        zig.contains("js_string.padStart("),
        "Expected js_string.padStart() call, got:\n{}",
        zig
    );
}

#[test]
fn test_arraylist_length_access() {
    // toReversed() returns ArrayList → .length should emit .items.len
    let js = r#"
export function getReversedLen() {
    const arr = [1, 2, 3];
    const rev = arr.toReversed();
    return rev.length;
}
"#;
    let zig = transpile_and_check(js, "test_arraylist_length_access");
    assert!(
        zig.contains(".items.len"),
        "Expected '.items.len' for ArrayList .length, got:\n{}",
        zig
    );
}

#[test]
fn test_arraylist_index_access() {
    // toReversed() returns ArrayList → [i] should emit .items[@as(usize, @intCast(i))]
    let js = r#"
export function getReversedFirst() {
    const arr = [1, 2, 3];
    const rev = arr.toReversed();
    return rev[0];
}
"#;
    let zig = transpile_and_check(js, "test_arraylist_index_access");
    assert!(
        zig.contains(".items[@as(usize, @intCast("),
        "Expected '.items[@as(usize, @intCast(...))]' for ArrayList index access, got:\n{}",
        zig
    );
}

#[test]
fn test_arraylist_needs_deinit() {
    // toReversed() returns ArrayList → needs defer .deinit()
    let js = r#"
export function testDeinit() {
    const arr = [1, 2, 3];
    const rev = arr.toReversed();
    return rev[0];
}
"#;
    let zig = transpile_and_check(js, "test_arraylist_needs_deinit");
    assert!(
        zig.contains("defer rev.deinit(js_allocator.allocator())"),
        "Expected 'defer rev.deinit(js_allocator.allocator())' for ArrayList variable, got:\n{}",
        zig
    );
}
