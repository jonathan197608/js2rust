// P1: in/instanceof, Date, Object static, labeled, spread

use super::common::*;

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
    let zig = transpile_and_check(js, "test_p1_in_operator");
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
    // obj instanceof Array → js_runtime.instanceOf(obj, "Array") (runtime dispatch for untyped obj)
    let js = r#"
function checkType(obj) {
return obj instanceof Array;
}
"#;
    let zig = transpile_and_assert(js, "test_p1_instanceof_operator");
    assert!(
        zig.contains("instanceOf"),
        "Expected instanceOf call in:\n{}",
        zig
    );
    assert!(
        zig.contains("\"Array\""),
        "Expected 'Array' string literal in:\n{}",
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
    let zig = transpile_and_check(js, "test_p1_date_now");
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
    let zig = transpile_and_check(js, "test_p1_date_parse");
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
    let zig = transpile_and_check(js, "test_p1_date_utc");
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
    let zig = transpile_and_check(js, "test_p1_date_instance_methods");
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
    let zig = transpile_and_assert(js, "test_p1_date_new_empty");
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
    let zig = transpile_and_assert(js, "test_p1_date_new_millis");
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
    let zig = transpile_and_assert(js, "test_p1_date_new_string");
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
    let zig = transpile_and_assert(js, "test_p1_date_new_multi_2args");
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
    let zig = transpile_and_assert(js, "test_p1_date_new_multi_3args");
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
    let zig = transpile_and_assert(js, "test_p1_date_new_multi_5args");
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
    let zig = transpile_and_assert(js, "test_p1_date_new_multi_7args");
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
    let zig = transpile_and_assert(js, "test_p1_date_new_multi_variable_args");
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
    let zig = transpile_and_check(js, "test_p1_object_keys");
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
    let zig = transpile_and_check(js, "test_p1_object_values");
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
    let zig = transpile_and_check(js, "test_p1_object_entries");
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
    let zig = transpile_and_check(js, "test_p1_object_assign");
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
    let zig = transpile_and_check(js, "test_p1_object_freeze");
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
    let zig = transpile_and_check(js, "test_p1_object_from_entries");
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
    let zig = transpile_and_assert(js, "test_p1_labeled_while");
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
    let zig = transpile_and_assert(js, "test_p1_labeled_for");
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
    let zig = transpile_and_assert(js, "test_p1_labeled_do_while");
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
    let zig = transpile_and_assert(js, "test_p1_labeled_for_of");
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
    let zig = transpile_and_assert(js, "test_p2_for_of_map_single_var");
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
    let zig = transpile_and_assert(js, "test_p2_for_of_map_destructure");
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
    let zig = transpile_and_assert(js, "test_p2_for_of_set");
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
    let zig = transpile_and_assert(js, "test_p2_for_of_string");
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
    let zig = transpile_and_assert(js, "test_p1_labeled_block");
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
    let zig = transpile_and_assert(js, "test_p1_spread_single");
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
    let zig = transpile_and_assert(js, "test_p1_spread_with_inline");
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
    let zig = transpile_and_assert(js, "test_p1_spread_multi");
    // spreadMerge appears twice: once in @TypeOf(return_expr), once in return body
    let merge_count = zig.matches("spreadMerge").count();
    assert_eq!(
        merge_count, 2,
        "Expected 2 spreadMerge calls, got {}:\n{}",
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
    let zig = transpile_and_assert(js, "test_p1_spread_multi_with_inline");
    // spreadMerge appears twice: once in @TypeOf(return_expr), once in return body
    // Each occurrence has 2 spreadMerge calls (nested)
    let merge_count = zig.matches("spreadMerge").count();
    assert_eq!(
        merge_count, 4,
        "Expected 4 spreadMerge calls, got {}:\n{}",
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
    let zig = transpile_and_check(js, "test_p1_spread_empty");
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
    let zig = transpile_and_assert(js, "test_p1_array_spread_simple");
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
    let zig = transpile_and_assert(js, "test_p1_array_spread_mixed");
    assert!(
        zig.contains("appendSlice"),
        "Expected appendSlice in:\n{}",
        zig
    );
    assert!(
        zig.contains("append(js_allocator.allocator()"),
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
    let zig = transpile_and_assert(js, "test_p1_array_spread_single");
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
    let zig = transpile_and_assert(js, "test_p1_array_spread_elision");
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
    let zig = transpile_and_assert(js, "test_p1_rest_param_and_call_spread");
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
    let zig = transpile_and_assert(js, "test_p1_call_spread");
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
