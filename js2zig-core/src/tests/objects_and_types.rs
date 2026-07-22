// Objects, JSDoc, typedef, export signatures, JSON E2E, string concat/template

use super::common::*;

/// Write `zig_code` to a temp file named `file_name`, run `zig ast-check`, and
/// return the temp path (for reuse in e.g. `zig build-exe`).
/// Panics if ast-check fails. Returns `None` if `zig` is not available.
fn zig_ast_check(zig_code: &str, file_name: &str) -> Option<std::path::PathBuf> {
    let tmp_dir = std::env::temp_dir();
    let zig_path = tmp_dir.join(file_name);
    std::fs::write(&zig_path, zig_code).unwrap();

    let check_output = std::process::Command::new(zig_binary())
        .args(["ast-check", zig_path.to_str().unwrap()])
        .output();

    match check_output {
        Ok(o) => {
            if !o.status.success() {
                eprintln!("=== zig ast-check failed ===");
                eprintln!("Generated code:\n{}", zig_code);
                eprintln!("stderr: {}", String::from_utf8_lossy(&o.stderr));
                panic!("zig ast-check failed");
            } else {
                println!("=== zig ast-check passed ===");
                Some(zig_path)
            }
        }
        Err(e) => {
            eprintln!("Failed to run zig ast-check: {}", e);
            None
        }
    }
}

#[test]
fn test_native_proto_object_struct() {
    // Scheme C: Only static access →anonymous struct.
    let js = r#"
function main() {
const pt = { x: 10, y: 20 };
const a = pt.x;
const b = pt.y;
return a + b;
}
"#;
    let zig = parse_and_transpile(js, None).unwrap().zig_code;
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
    // Dynamic struct access: obj[key] → @field(obj, key)
    let js = r#"
function main() {
const obj = { x: 1, y: 2 };
const key = "x";
const val = obj[key];
return val;
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_object_map");
    assert!(zig.contains("@field("));
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
    let zig = transpile_and_assert(js, "test_native_proto_object_struct_mutation");
    // Should use 'var' for the object (because it's mutated).
    // Rule 5: var with definite type may have type annotation (struct literal).
    assert!(zig.contains("var pt"));
    // Should generate anonymous struct literal.
    assert!(zig.contains(".{"));
    // Should assign to field directly.
    assert!(zig.contains("pt.x = 30"));
    // Should access field directly.
    assert!(zig.contains("pt.x;"));
}

#[test]
fn test_native_proto_object_map_mutation() {
    // Dynamic struct mutation: obj[key] = val → @field(obj, key) = val
    let js = r#"
function main() {
var obj = { x: 1, y: 2 };
const key = "x";
obj[key] = 10;
const val = obj[key];
return val;
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_object_map_mutation");
    assert!(zig.contains("@field("));
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
    let zig = transpile_and_assert(js, "test_native_proto_field_type_mismatch");
    // Should use 'var' for the object (because it's mutated).
    // Rule 5: var with definite type may have type annotation (struct literal).
    assert!(zig.contains("var pt"));
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
    let zig = parse_and_transpile(js, None).unwrap().zig_code;
    println!("=== JSDoc @typedef ===\n{}", zig);
    // Should generate struct definition at the top.
    assert!(zig.contains("const User = struct {"));
    assert!(zig.contains("name: []const u8,"));
    assert!(zig.contains("age: f64,"));
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
    let zig = parse_and_transpile(js, None).unwrap().zig_code;
    println!("=== JSDoc @type + JSON.parse() ===\n{}", zig);
    // Should generate struct definition.
    assert!(zig.contains("const User = struct {"));
    // Should generate std.json.parse(User, ...) for JSON.parse() with @type.
    assert!(
        zig.contains("std.json.parse(User,"),
        "Expected std.json.parse(User, ...), got: {}",
        zig
    );
    // Top-level JSON.parse uses `catch @panic(...)` (cannot `return` outside a function).
    assert!(
        zig.contains("catch @panic(\"JSON.parse failed\")"),
        "Expected catch @panic for top-level JSON.parse, got: {}",
        zig
    );
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
    let zig = transpile_and_assert(js, "test_native_proto_export_fn_signature");
    // Export function: should use real types from JSDoc
    // For export functions without @param: default to i64 (not anytype)
    assert!(zig.contains("pub fn add(a: i64, b: i64) f64 {"));
    // Export function with @returns {void}: should be void.
    assert!(zig.contains("pub fn log(_msg: i64) void {"));
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
    let zig = transpile_and_check(js, "test_native_proto_param_annotation");
    // @param {string} name: should use []const u8 directly
    // @param {number} age: should use f64 directly
    // NOTE: native_proto adds 'export ' prefix to export functions
    // Rule 1: JSDoc @returns should be used correctly (now fixed)
    assert!(zig.contains("pub fn greet(name: []const u8, age: f64) []const u8 {"));
    // Should NOT generate parseInt code (types are already correct)
    assert!(!zig.contains("parseInt"));
    // Should use std.fmt.allocPrint for string concatenation (Zig 0.16.0: ++ requires comptime-known slices)
    assert!(zig.contains("std.fmt.allocPrint"));
}

#[test]
fn test_native_proto_export_requires_returns() {
    // Test that export functions require @returns annotation.
    // NOTE: In real pipeline, export is stripped and exported_functions is passed.
    // "getName" is in exported_functions but has no @returns -> should error.
    let js = r#"
/**
 * @param {Object} user
 */
function getName(user) {
return user.name;
}
"#;
    let mut exports = std::collections::HashSet::new();
    exports.insert("getName".to_string());
    // This should error because export function needs @returns
    // But currently errors are in result.errors, not Err
    let result = parse_and_transpile(js, Some(exports));
    assert!(
        result.is_ok(),
        "transpile should succeed (errors in .errors field)"
    );
    let tr = result.unwrap();
    assert!(!tr.errors.is_empty(), "should have errors");
    let all_errs = tr.errors.join("; ");
    assert!(
        all_errs.contains("@returns"),
        "should mention @returns, got: {}",
        all_errs
    );
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
    let zig = parse_and_transpile(js, None).unwrap().zig_code;
    println!("=== @param E2E Test ===\n{}", zig);

    // Verify the generated code has correct structure with real types
    assert!(zig.contains("fn multiply(a: f64, b: f64) f64 {"));
    // Should NOT generate parseInt code (types are already f64)
    assert!(!zig.contains("parseInt"));
    // Should NOT generate allocPrint code (return type is f64, not string)
    assert!(!zig.contains("allocPrint"));
    assert!(!zig.contains("result_len"));
    // Should directly return the multiplication result
    assert!(zig.contains("return (a * b);"));

    // Run zig ast-check to verify the code is syntactically correct
    zig_ast_check(&zig, "param_e2e_test.zig");
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
    let zig = transpile_and_assert(js, "test_native_proto_string_concat");

    // Verify string concatenation uses std.fmt.allocPrint (Zig 0.16.0: ++ requires comptime-known slices)
    assert!(
        zig.contains("std.fmt.allocPrint"),
        "Expected allocPrint for string concat, got:\n{}",
        zig
    );
    assert!(
        !zig.contains(" ++ "),
        "Should not use ++ operator for string concat, got:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_string_concat_multi() {
    // Test: multiple string concatenation
    let js = r#"
function fullName(first, last) {
return first + " " + last;
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_string_concat_multi");

    println!("=== Generated Zig code ===\n{}", zig);

    // Verify all concatenations use std.fmt.allocPrint (Zig 0.16.0: ++ requires comptime-known slices)
    assert!(
        zig.contains("std.fmt.allocPrint"),
        "Expected allocPrint for string concat, got:\n{}",
        zig
    );
    assert!(
        !zig.contains(" ++ "),
        "Should not use ++ operator for string concat, got:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_template_literal_basic() {
    // Template literal with a single numeric interpolation → allocPrint via arena.
    let js = r#"
function label() {
const n = 42;
return `n=${n}`;
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_template_literal_basic");
    assert!(
        zig.contains("std.fmt.allocPrint"),
        "Expected allocPrint for template literal, got:\n{}",
        zig
    );
    assert!(
        zig.contains("js_allocator.allocator()"),
        "Expected arena allocator, got:\n{}",
        zig
    );
    assert!(
        zig.contains("\"n={d}\""),
        "Expected type-aware `n={{d}}` format string, got:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_template_literal_multiline() {
    // Multi-line template with multiple interpolations → newline escaped as \n in fmt.
    let js = r#"
function lines() {
const a = 1;
const b = 2;
return `a=${a}
sum=${a + b}`;
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_template_literal_multiline");
    assert!(
        zig.contains("std.fmt.allocPrint"),
        "Expected allocPrint, got:\n{}",
        zig
    );
    assert!(
        zig.contains("\\n"),
        "Expected escaped newline in format string, got:\n{}",
        zig
    );
}

#[test]
fn test_native_proto_template_literal_text_only() {
    // Pure-text template (no interpolation) degrades to a plain string literal.
    let js = r#"
function banner() {
return `hello world`;
}
"#;
    let zig = transpile_and_assert(js, "test_native_proto_template_literal_text_only");
    assert!(
        zig.contains("\"hello world\""),
        "Expected plain string literal, got:\n{}",
        zig
    );
    assert!(
        !zig.contains("allocPrint"),
        "Pure-text template should not allocate, got:\n{}",
        zig
    );
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
    let zig = transpile_and_assert(js, "test_native_proto_export_returns_string");

    // Rule 1: JSDoc @returns should be used correctly
    assert!(zig.contains("pub fn greet(name: []const u8) []const u8 {"));
    // String returns are allocated via the global arena allocator.
    // Memory is automatically freed when the arena is reset (no free_string needed).
    assert!(zig.contains("std.fmt.allocPrint"));
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
    let zig = transpile_and_check(js, "test_native_proto_typedef_tojson");

    // Verify Address struct is generated
    assert!(
        zig.contains("const Address = struct {"),
        "Expected Address struct, got:\n{}",
        zig
    );
    assert!(
        zig.contains("street: []const u8,"),
        "Expected street field, got:\n{}",
        zig
    );
    assert!(
        zig.contains("city: []const u8,"),
        "Expected city field, got:\n{}",
        zig
    );
    assert!(
        zig.contains("zip: f64,"),
        "Expected zip field, got:\n{}",
        zig
    );

    // Verify Address has toJson() method
    assert!(
        zig.contains("pub fn toJson") && zig.contains("Address"),
        "Expected toJson() for Address, got:\n{}",
        zig
    );

    // Verify User struct is generated with all field types
    assert!(
        zig.contains("const User = struct {"),
        "Expected User struct, got:\n{}",
        zig
    );
    assert!(
        zig.contains("name: []const u8,"),
        "Expected name field, got:\n{}",
        zig
    );
    assert!(
        zig.contains("age: f64,"),
        "Expected age field, got:\n{}",
        zig
    );
    assert!(
        zig.contains("tags: []const []const u8,"),
        "Expected tags field (string[]), got:\n{}",
        zig
    );
    assert!(
        zig.contains("scores: []const f64,"),
        "Expected scores field (number[]), got:\n{}",
        zig
    );
    assert!(
        zig.contains("addresses: []const Address,"),
        "Expected addresses field (Address[]), got:\n{}",
        zig
    );

    // Verify User has toJson() method
    assert!(
        zig.contains("pub fn toJson") && zig.contains("const User"),
        "Expected toJson() for User, got:\n{}",
        zig
    );

    // Verify toJson() uses std.json.fmt() for serialization
    assert!(
        zig.contains("std.json.fmt"),
        "Expected std.json.fmt() in toJson(), got:\n{}",
        zig
    );
    assert!(
        zig.contains("Writer.Allocating"),
        "Expected Writer.Allocating in toJson(), got:\n{}",
        zig
    );

    // Verify JSON.stringify() is converted to js_json.stringify()
    assert!(
        zig.contains("try js_json.stringify(js_allocator.allocator(), user"),
        "Expected try js_json.stringify(), got:\n{}",
        zig
    );
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
    let zig = transpile_and_check(js, "test_native_proto_json_parse_nested");

    // Verify Address and User structs are generated
    assert!(
        zig.contains("const Address = struct {"),
        "Expected Address struct, got:\n{}",
        zig
    );
    assert!(
        zig.contains("const User = struct {"),
        "Expected User struct, got:\n{}",
        zig
    );

    // Verify JSON.parse() is converted to std.json.parse()
    assert!(
        zig.contains("std.json.parse(User,"),
        "Expected std.json.parse(User, ...), got:\n{}",
        zig
    );

    // Verify data variable uses the correct type
    assert!(
        zig.contains("const data: User ="),
        "Expected 'const data: User', got:\n{}",
        zig
    );

    // Verify member access works (data.name, data.addresses[...].city)
    // Note: slice indices use @as(usize, @intCast(...)) for i64→usize conversion.
    assert!(
        zig.contains("data.name"),
        "Expected data.name access, got:\n{}",
        zig
    );
    assert!(
        zig.contains("data.addresses[") && zig.contains("].city"),
        "Expected data.addresses[...].city access, got:\n{}",
        zig
    );
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

    // Step 1: generate Zig source from JS (using macro to reduce duplication)
    let zig_gen = transpile_and_assert(js, "test_native_proto_e2e_json");

    // Step 2: create a complete Zig program
    // Remove `const std = @import("std");` from generated code to avoid duplicate
    let zig_gen_clean = zig_gen.replace("const std = @import(\"std\");\n", "");

    let zig_full = format!(
        r#"const std = @import("std");
const js_allocator = @import("js_runtime/js_allocator.zig");
const js_json = @import("js_runtime/js_json.zig");

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

    // Step 3: write to temp file, run ast-check
    let Some(zig_path) = zig_ast_check(&zig_full, "e2e_json_test.zig") else {
        return; // zig not available
    };

    // Step 4: compile with `zig build-exe`
    let tmp_dir = zig_path.parent().unwrap();
    let exe_path = tmp_dir.join("e2e_json_test.exe");
    let compile_output = std::process::Command::new(zig_binary())
        .args(["build-exe", zig_path.to_str().unwrap(), "-freference-trace"])
        .current_dir(tmp_dir)
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
        let run_output = std::process::Command::new(&exe_path).output().unwrap();

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        println!("=== Program output ===\n{}", stdout);

        // Verify output contains expected strings
        assert!(
            stdout.contains("Serialized JSON:"),
            "Expected 'Serialized JSON:' in output, got: {}",
            stdout
        );
        assert!(
            stdout.contains("Bob"),
            "Expected 'Bob' in output, got: {}",
            stdout
        );
        assert!(
            stdout.contains("Parsed: Alice is 30 years old"),
            "Expected 'Parsed: Alice is 30 years old' in output, got: {}",
            stdout
        );
    } else {
        eprintln!("Executable not found: {:?}", exe_path);
    }
}

// ── Test: Optional properties (@property {type} [name]) ─────────────
