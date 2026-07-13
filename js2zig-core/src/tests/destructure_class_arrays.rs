// Destructuring, class, String methods, Array higher-order, misc

use super::common::*;

#[test]
fn test_p2_destructure_object_basic() {
    // const {a, b} = obj → const a = obj.get("a"); const b = obj.get("b");
    let js = r#"
function basic(obj) {
const { a, b } = obj;
return a + b;
}
"#;
    let zig = transpile_and_assert(js, "test_p2_destructure_object_basic");
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
    let zig = transpile_and_check(js, "test_p2_destructure_object_with_defaults");
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
    let zig = transpile_and_assert(js, "test_p2_destructure_object_rename");
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
    let zig = transpile_and_check(js, "test_p2_destructure_object_mixed");
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
    let zig = transpile_and_assert(js, "test_p2_destructure_array_basic");
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
    let zig = transpile_and_check(js, "test_p2_destructure_array_with_defaults");
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
    let zig = transpile_and_assert(js, "test_p2_destructure_array_hole");
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
    let zig = transpile_and_assert(js, "test_p2_destructure_function_call_init");
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
    let zig = transpile_and_assert(js, "test_p2_nested_function_basic");
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
    let zig = transpile_and_assert(js, "test_p2_nested_function_capture_error");
    println!("=== Nested function capture Zig code ===\n{}", zig);

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
fn test_p2_nested_function_anytype_return() {
    // Rule 8: Nested function with anytype parameters whose return
    // depends on those parameters should use @TypeOf() via AnytypeReturn.
    // The nested function correctly gets @TypeOf(...) for its return type.
    // The outer function defaults to i64 with a Rule 8 warning because
    // AnytypeReturn cannot be propagated through call boundaries
    // (the nested function is not visible at the return-type position).
    let js = r#"
function outer() {
function add(a, b) {
    return a + b;
}
return add(1, 2);
}
"#;
    let result = parse_and_transpile(js, None).unwrap();
    let zig = result.zig_code;
    println!("=== Nested function (AnytypeReturn) Zig code ===\n{}", zig);

    // Verify: add's return type uses @TypeOf(return expression)
    assert!(
        zig.contains("@TypeOf("),
        "Expected @TypeOf() for AnytypeReturn in:\n{}",
        zig
    );

    // Verify: add is properly lifted as a struct
    assert!(
        zig.contains("const add = struct {"),
        "Expected 'const add = struct {{' in:\n{}",
        zig
    );
    assert!(
        zig.contains("add.call("),
        "Expected call rewriting to add.call() in:\n{}",
        zig
    );

    // The outer function gets a Rule 8 warning because it can't propagate
    // AnytypeReturn through the function call boundary. This is acceptable:
    // Zig's @TypeOf can't reference variables defined inside the function body.
    let rule8_errors: Vec<_> = result
        .errors
        .iter()
        .filter(|e| e.contains("Rule 8"))
        .collect();
    assert_eq!(
        rule8_errors.len(),
        1,
        "Expected exactly one Rule 8 error for 'outer', got: {:?}",
        rule8_errors
    );
    assert!(
        rule8_errors[0].contains("outer"),
        "Rule 8 error should mention 'outer': {:?}",
        rule8_errors
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
    let zig = transpile_and_assert(js, "test_native_proto_class_basic");
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
    let zig = transpile_and_assert(js, "test_native_proto_class_mixed_fields");
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
    let zig = transpile_and_assert(js, "test_native_proto_class_implicit_fields");
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
    let zig = transpile_and_assert(js, "test_native_proto_array_flat");
    println!("=== Array.flat Zig code ===\n{}", zig);

    // flat() without callback falls through to runtime js_array.flat()
    assert!(zig.contains("testFlat"), "Expected testFlat function");
    assert!(zig.contains("js_array.flat"), "Expected js_array.flat call");
}

#[test]
fn test_native_proto_array_flat_map() {
    let js = r#"
export function testFlatMap() {
const arr = [1, 2, 3];
return arr.flatMap((x) => x * 2);
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_array_flat_map");
    println!("=== Array.flatMap Zig code ===\n{}", zig);

    // flatMap with callback should be inlined (ArrayCallbackKind::FlatMap)
    assert!(zig.contains("testFlatMap"), "Expected testFlatMap function");
    assert!(zig.contains("__fmap"), "Expected __fmap inline expansion");
}

#[test]
fn test_native_proto_string_pad_start() {
    let js = r#"
export function testPadStart() {
const s = "42";
return s.padStart(5, "0");
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_string_pad_start");
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
    let zig = transpile_and_assert(js, "test_native_proto_string_pad_end");
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
    let zig = transpile_and_assert(js, "test_native_proto_string_substring");
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
    let zig = transpile_and_assert(js, "test_native_proto_string_substring_swap");
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
    let zig = transpile_and_assert(js, "test_native_proto_string_at");
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
    let zig_neg = transpile_and_assert(js_neg, "test_native_proto_string_at_neg");
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
    let zig = transpile_and_assert(js, "test_native_proto_string_code_point_at");
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
    let zig = transpile_and_check(js, "test_native_proto_object_has_own_struct");
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
    let zig = transpile_and_check(js, "test_native_proto_object_has_own_missing");
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
    let zig = transpile_and_check(js, "test_native_proto_array_filter");

    // Verify inline for-loop with ArrayList result
    assert!(zig.contains("blk_"), "Expected labeled block in:\n{}", zig);
    assert!(
        zig.contains("__filter: std.ArrayList("),
        "Expected __filter ArrayList var in:\n{}",
        zig
    );
    assert!(zig.contains("for ("), "Expected for loop in:\n{}", zig);
    assert!(
        zig.contains(".append(js_allocator.allocator()"),
        "Expected append in:\n{}",
        zig
    );
    assert!(
        zig.contains("break :blk_") && zig.contains("__filter"),
        "Expected break :blk_ __filter in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_array_some");
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
    let zig = transpile_and_check(js, "test_native_proto_array_every");
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
    let zig = transpile_and_check(js, "test_native_proto_array_some_block_body");
    assert!(
        zig.contains("break :blk_") && zig.contains(" true"),
        "Expected break :blk_ with true, got:\n{}",
        zig
    );
    assert!(
        zig.contains("break :blk_") && zig.contains(" false"),
        "Expected break :blk_ with false, got:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_array_every_block_body");
    assert!(
        zig.contains("break :blk_") && zig.contains(" true"),
        "Expected break :blk_ with true, got:\n{}",
        zig
    );
    assert!(
        zig.contains("break :blk_") && zig.contains(" false"),
        "Expected break :blk_ with false, got:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_array_concat");
    assert!(zig.contains("blk_"), "Expected labeled block in:\n{}", zig);
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
        zig.contains("break :blk_") && zig.contains("__concat"),
        "Expected break :blk_ __concat in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_array_find");
    assert!(
        zig.contains("break :blk_"),
        "Expected break :blk_ with value in:\n{}",
        zig
    );
    // find returns the element (x), not true/false
    assert!(
        zig.contains("break :blk_") && zig.contains(" x"),
        "Expected break :blk_ x in:\n{}",
        zig
    );
    assert!(
        zig.contains("break :blk_") && zig.contains(" undefined"),
        "Expected break :blk_ undefined fallback in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_array_find_block_body");
    assert!(
        zig.contains("break :blk_") && zig.contains(" x"),
        "Expected break :blk_ x in:\n{}",
        zig
    );
    assert!(
        zig.contains("break :blk_") && zig.contains(" undefined"),
        "Expected break :blk_ undefined fallback in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_array_find_index");
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
        zig.contains("break :blk_") && zig.contains(" -1"),
        "Expected break :blk_ -1 fallback in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_array_find_index_block_body");
    assert!(
        zig.contains("@intCast"),
        "Expected @intCast for index in:\n{}",
        zig
    );
    assert!(
        zig.contains("break :blk_") && zig.contains(" -1"),
        "Expected break :blk_ -1 fallback in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_array_find_last");
    assert!(
        zig.contains("var __i: usize = "),
        "Expected reverse loop in:\n{}",
        zig
    );
    assert!(
        zig.contains("break :blk_"),
        "Expected break :blk_ with value in:\n{}",
        zig
    );
    assert!(
        zig.contains("break :blk_") && zig.contains(" undefined"),
        "Expected break :blk_ undefined fallback in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_array_find_last_index");
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
        zig.contains("break :blk_") && zig.contains(" -1"),
        "Expected break :blk_ -1 fallback in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_array_fill");
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
    let zig = transpile_and_check(js, "test_native_proto_array_fill_range");
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
    let zig = transpile_and_check(js, "test_native_proto_array_at");

    // Verify labeled block with clamped index
    assert!(zig.contains("blk_"), "Expected labeled block in:\n{}", zig);
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
    let zig = transpile_and_check(js, "test_native_proto_array_lastindexof");

    // Verify backward while loop
    assert!(zig.contains("blk_"), "Expected labeled block in:\n{}", zig);
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
    let zig = transpile_and_check(js, "test_native_proto_array_copywithin");

    // Verify inline copy block
    assert!(zig.contains("blk_"), "Expected labeled block in:\n{}", zig);
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
        zig.contains("break :blk_") && zig.contains("&arr"),
        "Expected break :blk_ & in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_string_trimstart");

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
    let zig = transpile_and_check(js, "test_native_proto_string_trimend");

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
    let zig = transpile_and_check(js, "test_native_proto_string_lastindexof");

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
    let zig = transpile_and_assert(js, "test_native_proto_string_match_stub");

    assert!(
        zig.contains("js_string_regex.matchString(js_allocator.allocator(),"),
        "Expected js_string_regex.matchString(js_allocator.allocator(), for String.match() in:\n{}",
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
        zig.contains("host_regex.regex_search"),
        "Expected 'host_regex.regex_search' for String.search() in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_object_is");

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

// ── Test: Object.getOwnPropertyNames() ───────

#[test]
fn test_native_proto_object_getownpropertynames() {
    let js = r#"
export function getPropNames(obj) {
return Object.getOwnPropertyNames(obj);
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_object_getownpropertynames");
    println!("=== Object.getOwnPropertyNames Zig code ===\n{}", zig);

    // Should now emit actual runtime call, not @compileError
    assert!(
        zig.contains("getOwnPropertyNames"),
        "Expected getOwnPropertyNames in:\n{}",
        zig
    );
    assert!(
        !zig.contains("@compileError"),
        "Should not have @compileError for getOwnPropertyNames in:\n{}",
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
    let zig = transpile_and_check(js, "test_native_proto_anon_obj_type_returns");
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
    let zig = transpile_and_check(js, "test_native_proto_anon_obj_type_variable_access");
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
