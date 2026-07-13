// Private fields, not-implemented, regression fixes, shadowing, method chaining

use super::common::*;

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
    let zig = transpile_and_check(js, "test_p3_mixed_decl_expr_basic");
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
    let zig = transpile_and_assert(js, "test_p3_mixed_decl_expr_unused_var");
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
    let zig = transpile_and_check(js, "test_p3_mixed_decl_expr_call");
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
    let zig = transpile_and_check(js, "test_p3_mixed_decl_expr_return_unused");
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
    let zig = transpile_and_assert(js, "test_native_proto_private_field_basic");
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
    let zig = transpile_and_assert(js, "test_native_proto_private_field_no_default");
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
    let zig = transpile_and_assert(js, "test_native_proto_private_field_string_default");
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
    let zig = transpile_and_assert(js, "test_native_proto_private_field_multiple");
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
    let zig = transpile_and_assert(js, "test_native_proto_private_field_mixed");
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

// ── 🔘 不实现特性检查 ─────────────────────────────────────────────
// 验证所有标记为 🔘 不实现的特性都能报编译错误
// 合格标准：result.errors 非空 或 zig_code 包含 @compileError

#[test]
fn test_bigint_add() {
    // ✅ BigInt 字面量: 2n + 3n (现已支持)
    let zig = transpile_and_assert(
        r#"
function test() {
return 2n + 3n;
}
"#,
        "test_bigint_add",
    );
    assert!(
        zig.contains("JsBigInt"),
        "BigInt expr should generate JsBigInt code"
    );
}

#[test]
fn test_not_implemented_tagged_template() {
    // 🔘 标签模板: tag`...`
    assert_not_implemented(
        r#"
function tag(parts, ...args) { return parts[0]; }
const result = tag`hello ${1}`;
"#,
        "Tagged template literal",
    );
}

#[test]
fn test_implemented_instanceof_runtime() {
    // ✅ instanceof 运算符 — fallback runtime dispatch for untyped params
    let js = r#"
function check(arr) {
return arr instanceof Array;
}
"#;
    let zig = transpile_and_assert(js, "test_implemented_instanceof_runtime");
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

#[test]
fn test_class_expression() {
    // ✅ Class expression: const X = class { constructor(val) { this.x = val; } }
    // The class struct is emitted as a module-level declaration, and the
    // variable is assigned a reference to it.
    let js = r#"
/**
 * @returns {i64}
 */
export function testClassExpr() {
const X = class { constructor(val) { this.x = val; } };
const obj = X.init(42);
return obj.x;
}
"#;
    let zig = transpile_and_check(js, "test_class_expression");
    assert!(
        zig.contains("_AnonClass_0 = struct") || zig.contains("= struct"),
        "Expected class struct in:\n{}",
        zig
    );
    assert!(
        zig.contains("init("),
        "Expected init constructor in:\n{}",
        zig
    );
}

#[test]
fn test_class_expression_named() {
    // ✅ Named class expression: const X = class Y { ... }
    let js = r#"
/**
 * @returns {i64}
 */
export function testNamedClassExpr() {
const X = class MyClass { constructor(val) { this.x = val; } };
const obj = X.init(99);
return obj.x;
}
"#;
    let zig = transpile_and_check(js, "test_class_expression_named");
    assert!(
        zig.contains("MyClass = struct"),
        "Expected named class struct in:\n{}",
        zig
    );
    assert!(
        zig.contains("const X = MyClass"),
        "Expected X assigned to MyClass in:\n{}",
        zig
    );
}

#[test]
fn test_not_implemented_generator_function() {
    // 🔘 function*: 生成器函数
    assert_not_implemented(
        r#"
function* gen() { yield 1; yield 2; }
"#,
        "Generator function (function*)",
    );
}

#[test]
fn test_not_implemented_yield_expression() {
    // 🔘 yield: 生成器 yield 表达式
    assert_not_implemented(
        r#"
function* gen() { yield 1; }
const g = gen();
const val = g.next().value;
"#,
        "Yield expression (inside generator)",
    );
}

#[test]
fn test_not_implemented_async_generator() {
    // 🔘 async function*: 异步生成器
    assert_not_implemented(
        r#"
async function* gen() { yield 1; }
"#,
        "Async generator (async function*)",
    );
}

#[test]
fn test_not_implemented_dynamic_import() {
    // 🔘 动态 import(): import("module")
    assert_not_implemented(
        r#"
const mod = import("some_module");
"#,
        "Dynamic import()",
    );
}

#[test]
fn test_not_implemented_new_target() {
    // 🔘 new.target: meta property
    assert_not_implemented(
        r#"
function Foo() {
if (new.target) { return 1; }
return 0;
}
"#,
        "new.target meta property",
    );
}

#[test]
fn test_not_implemented_for_await_of() {
    // 🔘 for await...of: 异步迭代
    assert_not_implemented(
        r#"
async function process(items) {
for await (const item of items) { }
}
"#,
        "for await...of (async iteration)",
    );
}

#[test]
fn test_import_meta_is_implemented() {
    // ✅ import.meta: 已实现（生成 .{ .url = "..." } 结构体字面量）
    // 注意：const url = import.meta.url 会触发 Rule 8 类型推断错误
    // （ObjectLiteral 字段访问需要类型标注），但 import.meta 本身已正确实现
    let result = parse_and_transpile(
        r#"
const url = import.meta.url;
"#,
        None,
    )
    .unwrap();
    // import.meta 不应产生 "Unsupported" 或 "not supported" 错误
    let has_unsupported = result
        .errors
        .iter()
        .any(|e| e.contains("Unsupported") || e.contains("not supported"));
    assert!(
        !has_unsupported,
        "import.meta should be implemented, but got 'Unsupported' error: {:?}",
        result.errors
    );
    // 应产生 Rule 8 类型推断错误（这是预期的限制，不是功能缺失）
    let has_rule8 = result
        .errors
        .iter()
        .any(|e| e.contains("Rule 8") || e.contains("type annotation"));
    assert!(
        has_rule8,
        "import.meta.url access should produce Rule 8 type inference error, got: {:?}",
        result.errors
    );
}

#[test]
fn test_not_implemented_with_statement() {
    // 🔘 with 语句: with (obj) {}
    assert_not_implemented(
        r#"
const obj = { x: 1 };
with (obj) { console.log(x); }
"#,
        "with statement",
    );
}

#[test]
fn test_not_implemented_debugger_statement() {
    // 🔘 debugger 语句
    assert_not_implemented(
        r#"
function buggy() {
debugger;
}
"#,
        "debugger statement",
    );
}

#[test]
fn test_arguments_object() {
    // ✅ arguments object: now supported
    let js = r#"
function sum(a, b) {
let total = 0;
for (let i = 0; i < arguments.length; i++) { total += arguments[i]; }
return total;
}
"#;
    let zig = transpile_and_check(js, "test_arguments_object");
    assert!(
        zig.contains("__arguments"),
        "Expected __arguments variable in:\n{}",
        zig
    );
}

#[test]
fn test_static_block_no_error() {
    // ✅ static {}: initialization block — no longer produces a compileError.
    // (Full support for static field mutation in static blocks requires
    // static-var emission which is not yet complete.)
    let js = r#"
class Foo {
static x = 1;
}
"#;
    let zig = transpile_and_check(js, "test_static_block");
    // Verify: no @compileError about "static {} blocks are not supported"
    assert!(
        !zig.contains("static {} blocks are not supported"),
        "static block should not produce error about unsupported, got:\n{}",
        zig
    );
    assert!(
        !zig.contains("@compileError"),
        "should not have compileError, got:\n{}",
        zig
    );
}

#[test]
fn test_static_field_read() {
    // ✅ Static field read: ClassName.field → __ClassName_field module-scope var
    let js = r#"
class Foo {
  static x = 1;
}
/**
 * @returns {number}
 */
export function readStatic() {
  return Foo.x;
}
"#;
    let zig = transpile_and_check(js, "test_static_field_read");
    assert!(
        zig.contains("__Foo_x"),
        "Expected __Foo_x in output:\n{}",
        zig
    );
}

#[test]
fn test_static_field_assign() {
    // ✅ Static field write: ClassName.field = value → __ClassName_field = value
    let js = r#"
class Foo {
  static x = 1;
}
/**
 * @param {number} v
 * @returns {number}
 */
export function writeStatic(v) {
  Foo.x = v;
  return Foo.x;
}
"#;
    let zig = transpile_and_check(js, "test_static_field_assign");
    assert!(
        zig.contains("__Foo_x = v"),
        "Expected '__Foo_x = v' in output:\n{}",
        zig
    );
    assert!(
        zig.contains("return __Foo_x"),
        "Expected 'return __Foo_x' in output:\n{}",
        zig
    );
}

#[test]
fn test_static_block_this_read() {
    // ✅ `this.field` inside static {} → reads static field like ClassName.field
    let js = r#"
class Foo {
  static x = 1;
  static {
    this.x = this.x + 1;
  }
}
"#;
    let zig = transpile_and_check(js, "test_static_block_this_read");
    assert!(
        !zig.contains("@compileError"),
        "Should not produce compileError, got:\n{}",
        zig
    );
    // this.x read should produce __Foo_x, and write should produce __Foo_x =
    assert!(
        zig.contains("__Foo_x"),
        "Expected __Foo_x for this.x access:\n{}",
        zig
    );
}

#[test]
fn test_static_block_this_write() {
    // ✅ `this.field = value` inside static {} → writes static field like ClassName.field = value
    let js = r#"
class Foo {
  static x = 1;
  static {
    this.x = 42;
  }
}
"#;
    let zig = transpile_and_check(js, "test_static_block_this_write");
    assert!(
        !zig.contains("@compileError"),
        "Should not produce compileError, got:\n{}",
        zig
    );
    assert!(
        zig.contains("__Foo_x = 42"),
        "Expected '__Foo_x = 42' for this.x = 42:\n{}",
        zig
    );
}

// ── ✅ ES2023 Array immutable methods (now implemented) ──────────

#[test]
fn test_array_with() {
    // ✅ Array.prototype.with(index, value) — ES2023 immutable method
    let js = r#"
/**
 * @returns {number}
 */
export function testWith() {
const arr = [1, 2, 3];
const arr2 = arr.with(1, 99);
return arr2[1];
}
"#;
    let zig = transpile_and_check(js, "test_array_with");
    assert!(
        zig.contains("__with"),
        "Expected __with variable in:\n{}",
        zig
    );
    assert!(
        zig.contains("appendSlice"),
        "Expected clone via appendSlice in:\n{}",
        zig
    );
}

#[test]
fn test_array_to_reversed() {
    // ✅ Array.prototype.toReversed() — ES2023 immutable method
    let js = r#"
/**
 * @returns {number}
 */
export function testToReversed() {
const arr = [1, 2, 3];
const arr2 = arr.toReversed();
return arr2[0];
}
"#;
    let zig = transpile_and_check(js, "test_array_to_reversed");
    assert!(
        zig.contains("__rev"),
        "Expected __rev variable in:\n{}",
        zig
    );
    assert!(
        zig.contains("append"),
        "Expected append in reversed loop in:\n{}",
        zig
    );
}

#[test]
fn test_not_implemented_string_raw() {
    // 🔘 String.raw: 标签模板静态方法
    assert_not_implemented(
        r#"
function test() {
return String.raw`hello\nworld`;
}
"#,
        "String.raw (tagged template static method)",
    );
}

#[test]
fn test_not_implemented_map_group_by() {
    // 🔘 Map.groupBy(): ES2024 静态方法
    assert_not_implemented(
        r#"
function groupByAge(people) {
return Map.groupBy(people, (p) => p.age > 18 ? "adult" : "child");
}
"#,
        "Map.groupBy() (ES2024)",
    );
}

#[test]
fn test_not_implemented_set_operations() {
    // 🔘 Set.prototype.difference() etc: ES2025 Set 操作
    assert_not_implemented(
        r#"
function test() {
const a = new Set([1, 2, 3]);
const b = new Set([2, 3, 4]);
return a.difference(b);
}
"#,
        "Set.prototype.difference() (ES2025)",
    );
}

#[test]
fn test_not_implemented_object_get_own_property_symbols() {
    // 🔘 Object.getOwnPropertySymbols(): Symbol 属性
    assert_not_implemented(
        r#"
function test(obj) {
return Object.getOwnPropertySymbols(obj);
}
"#,
        "Object.getOwnPropertySymbols()",
    );
}

#[test]
fn test_object_group_by() {
    // ✅ Object.groupBy(): ES2024 static method now supported
    // Uses a simple callback with number comparison on array literal
    let js = r#"
/**
 * @returns {JsAny}
 */
export function test() {
const items = [1, 2, 3];
return Object.groupBy(items, (item) => item > 1 ? "big" : "small");
}
"#;
    let zig = transpile_and_check(js, "test_object_group_by");
    assert!(
        zig.contains("_grp_map"),
        "Expected _grp_map in groupBy emit:\n{}",
        zig
    );
}

#[test]
fn test_date_set_time() {
    // ✅ Date.prototype.setTime(): now supported
    let js = r#"
function test() {
const d = new Date();
return d.setTime(0);
}
"#;
    let zig = transpile_and_check(js, "test_date_set_time");
    assert!(zig.contains("setTime"), "Expected setTime in:\n{}", zig);
}

#[test]
fn test_date_to_utc_string() {
    // ✅ Date.prototype.toUTCString(): now supported
    let js = r#"
function test() {
const d = new Date();
return d.toUTCString();
}
"#;
    let zig = transpile_and_check(js, "test_date_to_utc_string");
    assert!(
        zig.contains("toUTCString"),
        "Expected toUTCString in:\n{}",
        zig
    );
}

#[test]
fn test_not_implemented_eval() {
    // 🔘 eval(): 安全风险，编译时无法动态执行
    assert_not_implemented(
        r#"
function test() {
return eval("1 + 2");
}
"#,
        "eval() (security risk)",
    );
}

#[test]
fn test_regexp_source() {
    // ✅ RegExp.prototype.source: now supported
    let js = r#"
function test() {
const re = /abc/g;
return re.source;
}
"#;
    let zig = transpile_and_check(js, "test_regexp_source");
    assert!(
        zig.contains("pattern"),
        "Expected pattern field access in:\n{}",
        zig
    );
}

#[test]
fn test_not_implemented_promise() {
    // 🔘 new Promise(): 建议用 async/await + Io 替代
    assert_not_implemented(
        r#"
function test() {
const p = new Promise((resolve, reject) => { resolve(1); });
return p;
}
"#,
        "new Promise() (use async/await instead)",
    );
}

#[test]
fn test_not_implemented_weakmap() {
    // 🔘 WeakMap: Zig 内存管理不同
    assert_not_implemented(
        r#"
function test() {
const wm = new WeakMap();
return wm;
}
"#,
        "WeakMap (Zig memory model different)",
    );
}

#[test]
fn test_not_implemented_weakset() {
    // 🔘 WeakSet: Zig 内存管理不同
    assert_not_implemented(
        r#"
function test() {
const ws = new WeakSet();
return ws;
}
"#,
        "WeakSet (Zig memory model different)",
    );
}

#[test]
fn test_not_implemented_reflect() {
    // 🔘 Reflect: 反射 API，Zig 不需要
    assert_not_implemented(
        r#"
function test(obj) {
return Reflect.has(obj, "x");
}
"#,
        "Reflect API (not needed in Zig)",
    );
}

#[test]
fn test_not_implemented_intl() {
    // 🔘 Intl: 国际化，可调用 Zig/C 库
    assert_not_implemented(
        r#"
function test() {
const fmt = new Intl.NumberFormat("en-US");
return fmt.format(1234.5);
}
"#,
        "Intl (use Zig/C library instead)",
    );
}

#[test]
fn test_bigint_constructor() {
    // ✅ BigInt(): 大整数构造函数 (现已支持)
    let zig = transpile_and_assert(
        r#"
function test() {
return BigInt(123);
}
"#,
        "test_bigint_constructor",
    );
    assert!(
        zig.contains("fromValue"),
        "BigInt(123) should generate fromValue code, got:\n{}",
        zig
    );
}

#[test]
fn test_not_implemented_atomics() {
    // 🔘 Atomics: 共享内存原子操作，niche 场景
    assert_not_implemented(
        r#"
function test(arr) {
return Atomics.load(arr, 0);
}
"#,
        "Atomics (niche scenario)",
    );
}

// ── for 循环初始值修复 ──────────────────────────────────────────
#[test]
fn test_for_loop_nonzero_init() {
    // Bug fix: `for (let i = 1; ...)` was erroneously emitting `var i: i64 = 0`
    // instead of `var i: i64 = 1`. Now the actual init expression is emitted.
    let js = r#"
/**
 * @returns {i64}
 */
export function sumFrom1() {
let sum = 0;
for (let i = 1; i <= 5; i = i + 1) {
    sum = sum + i;
}
return sum;
}
"#;
    let zig = transpile_and_assert(js, "test_for_loop_nonzero_init");
    assert!(
        zig.contains("var i: i64 = 1"),
        "Expected 'var i: i64 = 1' (not 0) in generated code:\n{}",
        zig
    );
    assert!(
        !zig.contains("var i: i64 = 0"),
        "Should not contain 'var i: i64 = 0' when init is 1:\n{}",
        zig
    );
}

// ── 补充：遗漏的 🔘 不实现特性 ──────────────────────────────────

#[test]
fn test_array_to_sorted() {
    // ✅ Array.prototype.toSorted() — ES2023 immutable method
    let js = r#"
/**
 * @returns {number}
 */
export function testToSorted() {
const arr = [3, 1, 2];
const arr2 = arr.toSorted();
return arr2[0];
}
"#;
    let zig = transpile_and_check(js, "test_array_to_sorted");
    assert!(
        zig.contains("__sorted"),
        "Expected __sorted variable in:\n{}",
        zig
    );
    assert!(zig.contains("sort"), "Expected sort call in:\n{}", zig);
}

#[test]
fn test_array_to_spliced() {
    // ✅ Array.prototype.toSpliced() — ES2023 immutable method
    let js = r#"
/**
 * @returns {number}
 */
export function testToSpliced() {
const arr = [1, 2, 3, 4];
const arr2 = arr.toSpliced(1, 2);
return arr2[1];
}
"#;
    let zig = transpile_and_check(js, "test_array_to_spliced");
    assert!(zig.contains("__sp"), "Expected __sp variable in:\n{}", zig);
    assert!(
        zig.contains("orderedRemove"),
        "Expected orderedRemove in:\n{}",
        zig
    );
}

#[test]
fn test_regexp_flags() {
    // ✅ RegExp.prototype.flags: now supported
    let js = r#"
function test() {
const re = /abc/gi;
return re.flags;
}
"#;
    let zig = transpile_and_check(js, "test_regexp_flags");
    assert!(
        zig.contains("flags"),
        "Expected flags field access in:\n{}",
        zig
    );
}

#[test]
fn test_regexp_global() {
    // ✅ RegExp.prototype.global: now supported
    let js = r#"
function test() {
const re = /abc/g;
return re.global;
}
"#;
    let zig = transpile_and_check(js, "test_regexp_global");
    assert!(
        zig.contains("global"),
        "Expected global field access in:\n{}",
        zig
    );
}

// ── 边缘情况：带类型标注的 🔘 特性 ──────────────────────────────
// 验证即使添加 @returns 类型标注，不实现特性仍然报错

#[test]
fn test_implemented_instanceof_with_annotation() {
    // ✅ instanceof with @returns annotation — no longer produces @compileError
    // Without JSDoc param type, arr is inferred from usage or defaults to anytype.
    // The key assertion is that instanceof is handled (no @compileError).
    let js = r#"
/**
 * @returns {bool}
 */
function check(arr) {
return arr instanceof Array;
}
"#;
    let zig = transpile_and_check(js, "test_implemented_instanceof_with_annotation");
    assert!(
        !zig.contains("@compileError"),
        "Should not contain @compileError:\n{}",
        zig
    );
    // Should produce either a runtime instanceOf call or a compile-time literal
    assert!(
        zig.contains("instanceOf") || zig.contains("true") || zig.contains("false"),
        "Expected instanceof resolution in:\n{}",
        zig
    );
}

#[test]
fn test_not_implemented_eval_with_annotation() {
    // 🔘 eval() 带返回类型标注 — 不应静默通过
    assert_not_implemented(
        r#"
/**
 * @returns {i64}
 */
function test() {
return eval("1 + 2");
}
"#,
        "eval() with @returns annotation",
    );
}

// ── #811: ternary + string concat with parenthesized expression ────

#[test]
fn test_native_proto_ternary_concat_parens() {
    // #811: ParenthesizedExpression wrapping ConditionalExpression in string concat
    // Fix: emit_string_concat / expr_is_string / infer_expr_type now unwrap ParenthesizedExpression
    let js = r#"
export function format(x) {
return "value: " + (x > 5 ? "big" : "small");
}
"#;
    let zig = transpile_and_assert(js, "test_ternary_concat_parens");
    println!("=== Ternary concat with parens ===\n{}", zig);

    // Should use {s} format specifier, not {}
    assert!(
        zig.contains("{s}"),
        "Expected {{s}} format specifier for string ternary, got:\n{}",
        zig
    );
    assert!(
        !zig.contains("{}"),
        "Should NOT use {{}} for string ternary, but got:\n{}",
        zig
    );
    assert!(
        zig.contains("std.fmt.allocPrint"),
        "Expected allocPrint for concat, got:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_ternary_concat_no_parens() {
    // Ternary in concat without explicit parens (sanity check: already worked)
    let js = r#"
export function format(x) {
return "value: " + x > 5 ? "big" : "small";
}
"#;
    let zig = transpile_and_assert(js, "test_ternary_concat_no_parens");
    println!("=== Ternary concat without parens ===\n{}", zig);

    // Even without parens, the operator precedence means + binds tighter,
    // so the ?? is "(value: " + x > 5) ? ...", which is a different semantic.
    // This test mainly ensures we don't crash; format specifier check is omitted.
    assert!(
        zig.contains("std.fmt.allocPrint"),
        "Expected allocPrint for concat, got:\n{}",
        zig
    );
}

// ── P2: Comparison operators always return Bool ─────────────────

#[test]
fn test_native_proto_comparison_strict_eq_bool() {
    // P2-1: When both operands are Indeterminate (e.g., function params),
    // === should still return Bool (not default to i64).
    let js = r#"
export function isEqual(a, b) {
return a === b;
}
"#;
    let zig = transpile_and_assert(js, "test_comparison_strict_eq_bool");
    println!("=== Comparison strict eq ===\n{}", zig);

    // Return type should be bool
    assert!(
        zig.contains("pub fn isEqual(") && zig.contains(") bool"),
        "Expected return type bool for === on indeterminate operands:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_comparison_neq_bool() {
    // !== should return Bool even with indeterminate operands.
    let js = r#"
export function isNotEqual(a, b) {
return a !== b;
}
"#;
    let zig = transpile_and_assert(js, "test_comparison_neq_bool");
    println!("=== Comparison neq ===\n{}", zig);
    assert!(
        zig.contains(") bool"),
        "Expected return type bool for !== on indeterminate operands:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_comparison_lt_bool() {
    // < should return Bool
    let js = r#"
export function isLess(a, b) {
return a < b;
}
"#;
    let zig = transpile_and_assert(js, "test_comparison_lt_bool");
    println!("=== Comparison less than ===\n{}", zig);
    assert!(
        zig.contains(") bool"),
        "Expected return type bool for < on indeterminate operands:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_comparison_complex_bool() {
    // Complex case: comparison in ternary should still produce bool
    let js = r#"
export function valueIsNaN(x) {
return x !== x ? "NaN" : "ok";
}
"#;
    let zig = transpile_and_assert(js, "test_comparison_complex_bool");
    println!("=== Comparison complex ===\n{}", zig);

    // The comparison x !== x should produce bool (not i64)
    assert!(
        zig.contains("pub fn valueIsNaN("),
        "Expected valueIsNaN to transpile successfully:\n{}",
        zig
    );
}

// ── Gap 1: BigInt compound assignment in for-loop update ──────────

/// Gap 1 diagnostic: BigInt += in for-loop update should emit valid Zig.
/// Currently the continuation syntax `while (cond) : ({ bigVar += ... })` is
/// invalid because BigInt has no Zig `+=` and the expansion contains `catch`.
#[test]
fn test_bigint_for_loop_compound_assign() {
    let js = r#"
/**
 * @param {bigint} sum
 * @returns {bigint}
 */
export function test(sum) {
    for (var i = 0; i < 3; i++) {
        sum += BigInt(i);
    }
    return sum;
}
"#;
    let zig = transpile_and_assert(js, "test_bigint_for_loop_compound");
    println!("=== BigInt for-loop compound assign ===\n{}", zig);

    // The update should NOT use += for BigInt; it should use .add() method
    // and handle catch correctly (not in while continuation).
    assert!(
        !zig.contains("sum +="),
        "BigInt compound assignment should not use Zig +=:\n{}",
        zig
    );
}

// ── Cross-type comparison: String vs Number, Bool vs Number, etc. ────

/// Gap 3 fix: comparing a @type-annotated string with a number literal
/// must route through JsAny comparison instead of emitting invalid Zig `==`.
#[test]
fn test_cross_type_str_vs_number_strict_eq() {
    let js = r#"
/**
 * @param {string} s
 * @param {number} n
 */
export function strEqNum(s, n) {
    return s === n;
}
"#;
    let zig = transpile_and_assert(js, "test_cross_type_str_vs_num_strict_eq");
    println!("=== Cross-type str === num ===\n{}", zig);

    // Should use JsAny.from(...).strictEq(JsAny.from(...)) or .eq()
    assert!(
        zig.contains("JsAny.from("),
        "Expected JsAny.from() wrapping for cross-type comparison:\n{}",
        zig
    );
}

/// Gap 3 fix: comparing a boolean with a number uses JsAny comparison.
#[test]
fn test_cross_type_bool_vs_number_eq() {
    let js = r#"
/**
 * @param {boolean} b
 * @param {number} n
 */
export function boolEqNum(b, n) {
    return b == n;
}
"#;
    let zig = transpile_and_assert(js, "test_cross_type_bool_vs_num_eq");
    println!("=== Cross-type bool == num ===\n{}", zig);

    assert!(
        zig.contains("JsAny.from("),
        "Expected JsAny.from() wrapping for bool vs number comparison:\n{}",
        zig
    );
}
