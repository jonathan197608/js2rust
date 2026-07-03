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
        zig.contains("js_string.normalize(js_allocator.allocator()"),
        "Expected 'js_string.normalize(js_allocator.allocator()' in:\n{}",
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
    let zig = transpile_and_check(js, "test_p6_string_to_locale_lower_case");
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
    let zig = transpile_and_check(js, "test_p6_string_locale_compare");
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
    let zig = transpile_and_check(js, "test_p6_string_from_char_code");
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
    let zig = transpile_and_check(js, "test_p6_string_from_code_point");
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
        zig.contains(r#"js_string.matchString(js_allocator.allocator(),"#),
        "Expected 'js_string.matchString(js_allocator.allocator(),' for String.match() in:\n{}",
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
        zig.contains("js_regexp.JsRegExp.init(js_allocator.allocator(),"),
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
    let js = r#"
/**
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
    // str.matchAll(/pattern/g) → js_string.matchAllString(alloc, str, "pattern")
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
    // str.matchAll(regexpVar) → js_string.matchAllString(alloc, str, regexpVar.pattern)
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
