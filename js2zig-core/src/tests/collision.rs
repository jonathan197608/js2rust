// Tests for export name collision disambiguation.
//
// When two JS files export a function with the same bare name,
// the pipeline should rename the CABI symbol to {fn}_{module}
// while keeping the internal per-file function name unchanged.

use crate::pipeline::gen_cabi_wrappers;
use crate::types::{NativeCabiExport, ZigType};
use std::collections::HashMap;

/// Helper: build a minimal NativeCabiExport for testing.
fn make_export(name: &str, ret: ZigType) -> NativeCabiExport {
    NativeCabiExport {
        name: name.to_string(),
        params: vec![],
        ret_type: ret,
        is_async: false,
        can_throw: false,
        ret_struct_name: None,
        ret_struct_fields: None,
    }
}

#[test]
fn test_cabi_wrappers_no_collision() {
    // Both names are unique → no disambiguation needed
    let name_to_module: HashMap<String, String> = HashMap::from([
        ("greet".to_string(), "main".to_string()),
        ("add".to_string(), "utils".to_string()),
    ]);
    let greet_exp = make_export("greet", ZigType::Str);
    let add_exp = make_export("add", ZigType::I64);
    let name_to_cabi: HashMap<String, &NativeCabiExport> = HashMap::from([
        ("greet".to_string(), &greet_exp),
        ("add".to_string(), &add_exp),
    ]);
    let cabi_rename = HashMap::new();

    let code = gen_cabi_wrappers(&name_to_module, &name_to_cabi, &cabi_rename);

    // String-returning greet: has Zig adapter + _cabi CABI wrapper
    assert!(
        code.contains("pub fn greet(") && code.contains("_main.greet("),
        "Expected 'pub fn greet(' with '_main.greet(' internal call, got:\n{code}"
    );
    assert!(
        code.contains("pub export fn greet_cabi("),
        "Expected 'pub export fn greet_cabi(' wrapper, got:\n{code}"
    );
    // i64-returning add: direct pub export fn (no _cabi suffix)
    assert!(
        code.contains("pub export fn add("),
        "Expected 'pub export fn add(' wrapper, got:\n{code}"
    );
    assert!(
        code.contains("_utils.add("),
        "Expected '_utils.add(' internal call, got:\n{code}"
    );
}

#[test]
fn test_cabi_wrappers_with_collision() {
    // Both modules export "process" → collision → disambiguate as process_utils / process_helpers
    let name_to_module: HashMap<String, String> = HashMap::from([
        ("process_utils".to_string(), "utils".to_string()),
        ("process_helpers".to_string(), "helpers".to_string()),
    ]);
    let utils_exp = make_export("process", ZigType::I64);
    let helpers_exp = make_export("process", ZigType::I64);
    let name_to_cabi: HashMap<String, &NativeCabiExport> = HashMap::from([
        ("process_utils".to_string(), &utils_exp),
        ("process_helpers".to_string(), &helpers_exp),
    ]);
    let cabi_rename: HashMap<String, String> = HashMap::from([
        ("process_utils".to_string(), "process".to_string()),
        ("process_helpers".to_string(), "process".to_string()),
    ]);

    let code = gen_cabi_wrappers(&name_to_module, &name_to_cabi, &cabi_rename);

    // Public declarations use disambiguated names
    assert!(
        code.contains("pub export fn process_utils("),
        "Expected 'pub export fn process_utils(' wrapper, got:\n{code}"
    );
    assert!(
        code.contains("pub export fn process_helpers("),
        "Expected 'pub export fn process_helpers(' wrapper, got:\n{code}"
    );

    // Internal calls use bare name inside the module
    assert!(
        code.contains("_utils.process("),
        "Expected '_utils.process(' internal call, got:\n{code}"
    );
    assert!(
        code.contains("_helpers.process("),
        "Expected '_helpers.process(' internal call, got:\n{code}"
    );

    // For i64 returns, the pub export fn itself is the CABI export
    // (no separate comptime @export block like string returns have).
    // The symbol name is "process_utils" / "process_helpers" directly.
    // Verify both disambiguated wrapper functions exist.
    let utils_count = code.matches("pub export fn process_utils(").count();
    let helpers_count = code.matches("pub export fn process_helpers(").count();
    assert_eq!(
        utils_count, 1,
        "Expected exactly 1 'pub export fn process_utils(' wrapper, found {utils_count}"
    );
    assert_eq!(
        helpers_count, 1,
        "Expected exactly 1 'pub export fn process_helpers(' wrapper, found {helpers_count}"
    );
}

#[test]
fn test_cabi_wrappers_collision_string_return() {
    // Collision with string-returning functions (which have _cabi suffix + StrRet)
    let name_to_module: HashMap<String, String> = HashMap::from([
        ("greet_mod_a".to_string(), "mod_a".to_string()),
        ("greet_mod_b".to_string(), "mod_b".to_string()),
    ]);
    let a_exp = make_export("greet", ZigType::Str);
    let b_exp = make_export("greet", ZigType::Str);
    let name_to_cabi: HashMap<String, &NativeCabiExport> = HashMap::from([
        ("greet_mod_a".to_string(), &a_exp),
        ("greet_mod_b".to_string(), &b_exp),
    ]);
    let cabi_rename: HashMap<String, String> = HashMap::from([
        ("greet_mod_a".to_string(), "greet".to_string()),
        ("greet_mod_b".to_string(), "greet".to_string()),
    ]);

    let code = gen_cabi_wrappers(&name_to_module, &name_to_cabi, &cabi_rename);

    // String return: should have pub fn + pub export fn _cabi + comptime @export
    assert!(
        code.contains("pub fn greet_mod_a("),
        "Expected 'pub fn greet_mod_a(' adapter, got:\n{code}"
    );
    assert!(
        code.contains("pub export fn greet_mod_a_cabi("),
        "Expected 'pub export fn greet_mod_a_cabi(' wrapper, got:\n{code}"
    );
    assert!(
        code.contains("_mod_a.greet("),
        "Expected '_mod_a.greet(' internal call, got:\n{code}"
    );
    assert!(
        code.contains("_mod_b.greet("),
        "Expected '_mod_b.greet(' internal call, got:\n{code}"
    );
    assert!(
        code.contains(r#".name = "greet_mod_a""#),
        "Expected @export name 'greet_mod_a', got:\n{code}"
    );
    assert!(
        code.contains(r#".name = "greet_mod_b""#),
        "Expected @export name 'greet_mod_b', got:\n{code}"
    );
}
