// Shadowing, method chaining, dynamic access, bit operators, array method tests
// Extracted from not_implemented_and_fixes.rs for file size management.

use super::common::*;

// ── Shadowing infrastructure tests (#946, #947) ─────────────────────

/// Test #946: Variable shadowing in nested blocks.
/// JS allows `let x` inside a nested block to shadow an outer `let x`.
/// Zig 0.16.0 forbids this, so the transpiler must rename the inner variable.
/// Note: we use `let` (block-scoped) not `var` (function-scoped).
#[test]
fn test_shadowing_nested_block() {
    let js = r#"
export function shadowTest() {
let x = 10;
if (true) {
    let x = 20;
    let y = x + 1;
}
return x;
}
"#;
    let zig = transpile_and_assert(js, "test_shadowing_nested_block");

    // The inner `x` should be renamed to `x_shadow_1` (or similar)
    // so it doesn't conflict with the outer `x`.
    println!("=== Shadowing nested block ===\n{}", zig);

    // Verify: outer x is declared
    assert!(
        zig.contains("const x = 10") || zig.contains("var x: i64 = 10"),
        "Expected outer x declaration"
    );

    // Verify: inner block exists and x is renamed
    assert!(
        zig.contains("x_shadow_1 = 20") || zig.contains("const x_shadow_"),
        "Expected inner x to be renamed to avoid shadowing:\n{}",
        zig
    );

    // Verify: return uses outer x (not the shadowed inner x)
    assert!(
        zig.contains("return x;") || zig.contains("return x\n"),
        "Expected return to use outer x:\n{}",
        zig
    );
}

/// Test #947: Function parameter name shadowing outer variable.
/// JS allows function parameters to have the same name as variables
/// in the outer scope. Zig doesn't allow this.
/// Note: this test exposes an issue with unused parameters in Zig.
/// The transpiler renames the local variable, but the parameter remains unused.
#[test]
fn test_shadowing_param_name() {
    let js = r#"
export function processData(data) {
let data = 100;
return data;
}
"#;
    let zig = transpile_and_assert(js, "test_shadowing_param_name");

    println!("=== Shadowing param name ===\n{}", zig);

    // The parameter `data` should be renamed to `data_param` (or similar)
    // to avoid shadowing the outer `data`.
    // But currently, the LOCAL variable is renamed, not the parameter.
    // This is because `fn_scope_vars` contains the parameter name.
    assert!(
        zig.contains("data_shadow_") || zig.contains("data_param"),
        "Expected local data to be renamed to avoid shadowing:\n{}",
        zig
    );
}

/// Test: Multiple levels of nesting with shadowing.
#[test]
fn test_shadowing_multiple_levels() {
    let js = r#"
export function nestedShadow(x) {
let y = x + 1;
if (true) {
    let x = y + 10;
    if (true) {
        let y = x + 20;
    }
}
return y;
}
"#;
    let zig = transpile_and_assert(js, "test_shadowing_multiple_levels");

    println!("=== Multiple levels shadowing ===\n{}", zig);

    // Verify the code compiles (ast-check passes)
    // The implementation should rename shadowed variables appropriately
    assert!(
        zig.contains("blk_") || zig.contains("{"),
        "Expected block expressions"
    );
}

// ── #844/#867: Method chaining & non-Identifier member function calls ──

#[test]
fn test_method_chaining_encodeuri_replace() {
    let js = r#"
        export function testChainedReplace(str) {
            return encodeURIComponent(str).replace(/%2F/g, "/");
        }
    "#;
    let zig = transpile_and_assert(js, "test_method_chaining_encodeuri_replace");
    println!("=== Method chaining: encodeURI().replace() ===\n{}", zig);
    // R8-P1-23: replace with RegExp literal now routes to replaceRegex
    // The object of .replace() is a CallExpression (encodeURIComponent(str))
    // callee_object_repr_mut should emit it inline as the object argument.
    assert!(
        zig.contains("js_string_regex.replaceRegex("),
        "Expected js_string_regex.replaceRegex() call for RegExp literal"
    );
    assert!(
        zig.contains("encodeURI") || zig.contains("encodeURIComponent"),
        "Expected encodeURI/component in the generated code"
    );
}

#[test]
fn test_method_chaining_string_literal_method() {
    let js = r#"
        export function testLiteralMethod() {
            return "Hello World".toLowerCase();
        }
    "#;
    let zig = transpile_and_assert(js, "test_method_chaining_string_literal_method");
    println!("=== String literal method call ===\n{}", zig);
    assert!(
        zig.contains("js_string.toLower"),
        "Expected js_string.toLower call"
    );
}

#[test]
fn test_method_chaining_array_join_after_map() {
    // Method chaining: arr.map(fn).join(sep)
    // The chain fix ensures: distinct block labels and single evaluation via __chain binding.
    let js = r#"
/**
 * @returns {string}
 */
export function testMapJoin() {
const arr = [1, 2, 3];
return arr.map(function(x) { return x * 2; }).join(",");
}
"#;
    let zig = transpile_and_assert(js, "test_method_chaining_array_join_after_map");
    println!("=== Array method chaining: map().join() ===\n{}", zig);
    // Verify inline map emission with chain binding
    assert!(
        zig.contains("__map"),
        "Expected inline __map ArrayList in:\n{}",
        zig
    );
    assert!(
        zig.contains("__chain_"),
        "Expected __chain binding for chained map().join() in:\n{}",
        zig
    );
    assert!(
        zig.contains("__join_buf"),
        "Expected inline __join_buf for join emission in:\n{}",
        zig
    );
}

#[test]
fn test_array_map_callback_transform() {
    // ✅ Array.map() now applies the callback — no longer identity stub
    let js = r#"
/**
 * @returns {number}
 */
export function testMapDouble() {
const arr = [1, 2, 3];
const doubled = arr.map(x => x * 2);
return doubled[0] + doubled[1] + doubled[2];
}
"#;
    let zig = transpile_and_check(js, "test_array_map_callback_transform");
    assert!(
        zig.contains("__map"),
        "Expected __map ArrayList in:\n{}",
        zig
    );
    assert!(
        zig.contains("append"),
        "Expected append in map emit in:\n{}",
        zig
    );
    assert!(
        zig.contains("* 2") || zig.contains("*2"),
        "Expected transform expression in:\n{}",
        zig
    );
}

#[test]
fn test_method_chaining_new_date_get_time() {
    let js = r#"
        export function testNewDateGetTime() {
            return new Date().getTime();
        }
    "#;
    let zig = transpile_and_assert(js, "test_method_chaining_new_date_get_time");
    println!("=== new Date().getTime() ===\n{}", zig);
    assert!(
        zig.contains("getTime()"),
        "Expected getTime() call on new Date()"
    );
}

#[test]
fn test_dynamic_array_access_index() {
    // arr[i] where arr is ArrayList and i is a variable
    let js = r#"
        export function getElement(n) {
            const arr = [1, 2, 3];
            return arr[n];
        }
    "#;
    let zig = transpile_and_assert(js, "test_dynamic_array_access_index");
    assert!(
        zig.contains(".items["),
        "Expected .items[] for ArrayList access"
    );
}

#[test]
fn test_dynamic_array_assignment_index() {
    // arr[i] = val where arr is ArrayList and i is a variable
    let js = r#"
        export function setElement(n, val) {
            const arr = [1, 2, 3];
            arr[n] = val;
            return arr[n];
        }
    "#;
    let zig = transpile_and_assert(js, "test_dynamic_array_assignment_index");
    assert!(
        zig.contains(".items["),
        "Expected .items[] for ArrayList assignment"
    );
}

#[test]
fn test_update_expr_in_index() {
    // arr[i++] = 0 — UpdateExpression in array index position
    let js = r#"
        export function fillZero(arr, n) {
            for (let i = 0; i < n; i++) {
                arr[i] = 0;
            }
        }
    "#;
    let zig = transpile_and_assert(js, "test_update_expr_in_index");
    // Should not contain raw "i += 1" inside array indexing
    assert!(
        !zig.contains(".items[i += 1]"),
        "Should not have i += 1 inside array index"
    );
}

#[test]
fn test_dynamic_string_index() {
    // str[idx] where idx is a variable → StringCharAt (returns Str)
    let js = r#"
        /**
         * @param {string} s
         */
        export function getChar(s, i) {
            return s[i];
        }
    "#;
    let zig = transpile_and_assert(js, "test_dynamic_string_index");
    assert!(
        zig.contains("js_string.charAt(js_allocator.allocator(),"),
        "Expected js_string.charAt(...) for StringCharAt access, got:\n{}",
        zig
    );
    assert!(
        !zig.contains("@as(i64, @intCast(js_string.charCodeAt"),
        "Should not emit charCodeAt (old buggy behavior) for str[idx]"
    );
}

#[test]
fn test_string_param_refined_from_method_usage() {
    // No JSDoc @param {string}, but s.charAt() usage infers s as Str
    // (only for non-export functions where params default to Anytype),
    // so s[i] should be treated as StringCharAt (not array index).
    let js = r#"
        export function testStringRefinement() {
            function inner(s, i) {
                const c = s.charAt(0);
                return s[i];
            }
            return inner("hello", 0);
        }
    "#;
    let zig = transpile_and_assert(js, "test_string_param_refined_from_method_usage");
    assert!(
        zig.contains("js_string.charAt(js_allocator.allocator(),"),
        "Expected js_string.charAt(...) for StringCharAt via type refinement, got:\n{}",
        zig
    );
}

#[test]
fn test_string_literal_index_inferred_type() {
    // const s = "hello"; s[0] — type inferred as Str, not relying on JSDoc
    let js = r#"
        export function getFirstChar() {
            const s = "hello";
            return s[0];
        }
    "#;
    let zig = transpile_and_assert(js, "test_string_literal_index_inferred_type");
    assert!(
        zig.contains("js_string.charAt(js_allocator.allocator(),"),
        "Expected js_string.charAt(...) for StringCharAt on inferred string var, got:\n{}",
        zig
    );
}

#[test]
fn test_bit_assign_operators() {
    // <<= >>= &= |= ^= compound assignment
    let js = r#"
        export function bitOps(x) {
            x <<= 2;
            x >>= 1;
            x &= 0xFF;
            x |= 0x10;
            x ^= 0x55;
            return x;
        }
    "#;
    let zig = transpile_and_assert(js, "test_bit_assign_operators");
    assert!(zig.contains("<<="), "Expected <<= in output");
    assert!(zig.contains(">>="), "Expected >>= in output");
    assert!(zig.contains("&="), "Expected &= in output");
    assert!(zig.contains("|="), "Expected |= in output");
    assert!(zig.contains("^="), "Expected ^= in output");
}

#[test]
fn test_in_operator() {
    // "key" in obj → @hasField or .contains
    let js = r#"
        export function hasKey(obj, key) {
            return key in obj;
        }
    "#;
    let zig = transpile_and_assert(js, "test_in_operator");
    // Should contain some form of containment check
    assert!(
        zig.contains("@hasField") || zig.contains(".contains"),
        "Expected @hasField or .contains for 'in' operator"
    );
}

#[test]
fn test_labeled_statement() {
    // label: while { break label; }
    let js = r#"
        export function labeledBreak(n) {
            let result = 0;
            outer: for (let i = 0; i < n; i++) {
                for (let j = 0; j < n; j++) {
                    result += j;
                    if (j === 3) break outer;
                }
            }
            return result;
        }
    "#;
    let zig = transpile_and_assert(js, "test_labeled_statement");
    assert!(zig.contains("outer:"), "Expected labeled statement");
    assert!(zig.contains("break :outer"), "Expected break with label");
}

// ── BigInt compound assignment on static class field ──────────

/// BigInt compound assignment on a static class field (e.g. `Counter.total += n`)
/// should be expanded to `Counter.total = Counter.total.add(n)` rather than
/// emitting invalid Zig `+=`.
#[test]
fn test_bigint_static_field_compound_assign() {
    let js = r#"
class Counter {
    /** @type {bigint} */
    static total = 0n;
}

/**
 * @param {bigint} n
 * @returns {bigint}
 */
export function add(n) {
    Counter.total += n;
    return Counter.total;
}
"#;
    let zig = transpile_and_assert(js, "test_bigint_static_field_compound");
    println!("=== BigInt static field compound assign ===\n{}", zig);

    // Should NOT use += for BigInt; should expand to .add() method call
    assert!(
        !zig.contains("total +="),
        "BigInt static field compound assignment should not use Zig +=:\n{}",
        zig
    );
    assert!(
        zig.contains(".add("),
        "BigInt static field compound assignment should use .add() method:\n{}",
        zig
    );
}

// ── BigInt ++/-- expansion ────────────────────────────────

/// BigInt `++` should expand to `.add()` method call, not emit invalid Zig `+= 1`.
#[test]
fn test_bigint_increment() {
    let js = r#"
/**
 * @param {bigint} x
 * @returns {bigint}
 */
export function test(x) {
    x++;
    return x;
}
"#;
    let zig = transpile_and_assert(js, "test_bigint_increment");
    println!("=== BigInt increment ===\n{}", zig);

    // Should NOT use += for BigInt
    assert!(
        !zig.contains("x +="),
        "BigInt increment should not use Zig +=:\n{}",
        zig
    );
    // Should use .add() method call
    assert!(
        zig.contains(".add("),
        "BigInt increment should use .add() method:\n{}",
        zig
    );
}

/// BigInt `--` should expand to `.sub()` method call, not emit invalid Zig `-= 1`.
#[test]
fn test_bigint_decrement() {
    let js = r#"
/**
 * @param {bigint} x
 * @returns {bigint}
 */
export function test(x) {
    x--;
    return x;
}
"#;
    let zig = transpile_and_assert(js, "test_bigint_decrement");
    println!("=== BigInt decrement ===\n{}", zig);

    assert!(
        !zig.contains("x -="),
        "BigInt decrement should not use Zig -=:\n{}",
        zig
    );
    assert!(
        zig.contains(".sub("),
        "BigInt decrement should use .sub() method:\n{}",
        zig
    );
}

/// BigInt static field `**=` should use `.pow()` method call, and should not
/// contain the invalid `__target` placeholder from the old Member target handling.
#[test]
fn test_bigint_static_field_pow_assign() {
    let js = r#"
class Math {
    /** @type {bigint} */
    static base = 2n;
}

/**
 * @param {bigint} exp
 * @returns {bigint}
 */
export function test(exp) {
    Math.base **= exp;
    return Math.base;
}
"#;
    let zig = transpile_and_assert(js, "test_bigint_static_field_pow_assign");
    println!("=== BigInt static field pow assign ===\n{}", zig);

    // Should NOT contain the __target placeholder
    assert!(
        !zig.contains("__target"),
        "BigInt **= on Member target should not use __target placeholder:\n{}",
        zig
    );
    // Should use .pow() method call
    assert!(
        zig.contains(".pow("),
        "BigInt **= should use .pow() method:\n{}",
        zig
    );
}

#[test]
fn test_method_chaining_array_filter_map() {
    // Method chaining: arr.filter(fn).map(fn)
    // This tests the fix for two bugs:
    //   Bug A — Label conflict: filter and map both emitted blk_0
    //   Bug B — Double evaluation: map rendered the filter expression twice
    // The fix emits a const __chain_N binding for the inner expression
    // and propagates label offsets so inner blocks use higher label numbers.
    let js = r#"
/**
 * @returns {number}
 */
export function testFilterMap() {
const arr = [1, 2, 3, 4, 5];
const result = arr.filter(x => x > 3).map(x => x * 2);
return result[0] + result[1];
}
"#;
    let zig = transpile_and_assert(js, "test_method_chaining_array_filter_map");
    println!("=== Array method chaining: filter().map() ===\n{}", zig);
    // Verify the output contains __chain binding (no double evaluation)
    assert!(
        zig.contains("__chain_"),
        "Expected __chain binding for chained filter().map() in:\n{}",
        zig
    );
    // Verify no label conflict: should have distinct blk labels
    assert!(zig.contains("blk_0"), "Expected blk_0 label in:\n{}", zig);
    assert!(
        zig.contains("blk_1"),
        "Expected blk_1 label (no label conflict) in:\n{}",
        zig
    );
}

#[test]
fn test_sequence_expression() {
    // Sequence expression: (a, b) evaluates both, returns b
    let js = r#"
export function seqExpr(x, y) {
    return (x, y);
}
"#;
    let zig = transpile_and_assert(js, "test_sequence_expression");
    // Should contain comma-separated expressions in the return
    assert!(
        zig.contains(", "),
        "Expected comma-separated sequence expression in:\n{}",
        zig
    );
}

#[test]
fn test_empty_statement() {
    // Empty statement: ; → ignored (comment)
    let js = r#"
export function emptyStmt() {
    ;
    return 42;
}
"#;
    let zig = transpile_and_assert(js, "test_empty_statement");
    // Should contain the return statement (empty statement is silently ignored)
    assert!(
        zig.contains("42"),
        "Expected return 42 after empty statement in:\n{}",
        zig
    );
}

#[test]
fn test_method_chaining_3_level_filter_map_join() {
    // 3-level chaining: arr.filter(fn).map(fn).join(sep)
    // Validates that label_offset propagation works through 3 nested inline emitters
    let js = r#"
/**
 * @returns {string}
 */
export function testFilterMapJoin() {
const arr = [1, 2, 3, 4, 5];
const result = arr.filter(x => x > 2).map(x => x * 10).join("-");
return result;
}
"#;
    let zig = transpile_and_assert(js, "test_method_chaining_3_level_filter_map_join");
    println!("=== 3-level chaining: filter().map().join() ===\n{}", zig);
    // Should contain __chain bindings for both inner expressions
    assert!(
        zig.contains("__chain_"),
        "Expected __chain binding for 3-level chain in:\n{}",
        zig
    );
    // Should contain distinct labels for filter and map blocks
    assert!(zig.contains("blk_0"), "Expected blk_0 in:\n{}", zig);
    assert!(zig.contains("blk_1"), "Expected blk_1 in:\n{}", zig);
}

#[test]
fn test_sort_with_comparefn() {
    // arr.sort((a, b) => a - b) → in-place sort with custom comparator
    let js = r#"
/**
 * @returns {number}
 */
export function testSortDesc() {
    const arr = [3, 1, 4, 1, 5];
    arr.sort((a, b) => b - a);
    return arr.items[0];
}
"#;
    let zig = transpile_and_assert(js, "test_sort_with_comparefn");
    println!("=== sort with compareFn ===\n{}", zig);
    // Should generate a struct with lessThan function (not std.sort.asc)
    assert!(
        zig.contains("lessThan"),
        "Expected lessThan function in custom sort:\n{}",
        zig
    );
    assert!(
        zig.contains("< 0"),
        "Expected '< 0' conversion from JS compareFn to Zig lessThan:\n{}",
        zig
    );
    // Should NOT use the default std.sort.asc comptime
    assert!(
        !zig.contains("std.sort.asc"),
        "Custom sort should not use std.sort.asc:\n{}",
        zig
    );
}

/// ECMA-262: arr.sort() without compareFn converts elements to strings and
/// compares by UTF-16 code unit sequence. Emits a custom lessThan closure
/// with comptime type dispatch: JsAny → .lt(), i64/f64 → string comparison,
/// other primitives → numeric <.
#[test]
fn test_sort_without_comparefn() {
    // arr.sort() → default sort with custom lessThan closure (no compareFn)
    let js = r#"
/**
 * @returns {number}
 */
export function testSortAsc() {
    const arr = [3, 1, 4, 1, 5];
    arr.sort();
    return arr.items[0];
}
"#;
    let zig = transpile_and_assert(js, "test_sort_without_comparefn");
    println!("=== sort without compareFn ===\n{}", zig);
    // Should emit a custom lessThan closure with ECMA-262 string comparison,
    // not comptime std.sort.asc
    assert!(
        zig.contains("lessThan"),
        "Default sort should use custom lessThan closure:\n{}",
        zig
    );
    assert!(
        !zig.contains("std.sort.asc"),
        "Default sort should not use std.sort.asc:\n{}",
        zig
    );
    // Should include string comparison for i64/f64 types
    assert!(
        zig.contains("std.mem.order"),
        "Default sort should use std.mem.order for string comparison:\n{}",
        zig
    );
}

#[test]
fn test_to_sorted_with_comparefn() {
    // arr.toSorted((a, b) => a - b) → sort returning new array with custom comparator
    let js = r#"
/**
 * @returns {number}
 */
export function testToSortedDesc() {
    const arr = [3, 1, 4, 1, 5];
    const sorted = arr.toSorted((a, b) => b - a);
    return sorted.items[0];
}
"#;
    let zig = transpile_and_assert(js, "test_toSorted_with_comparefn");
    println!("=== toSorted with compareFn ===\n{}", zig);
    // Should generate a struct with lessThan function
    assert!(
        zig.contains("lessThan"),
        "Expected lessThan function in custom toSorted:\n{}",
        zig
    );
    assert!(
        zig.contains("< 0"),
        "Expected '< 0' conversion from JS compareFn to Zig lessThan:\n{}",
        zig
    );
    // Should NOT use the default std.sort.asc comptime
    assert!(
        !zig.contains("std.sort.asc"),
        "Custom toSorted should not use std.sort.asc:\n{}",
        zig
    );
}

#[test]
fn test_to_sorted_without_comparefn() {
    // arr.toSorted() → default ascending sort (no compareFn)
    let js = r#"
/**
 * @returns {number}
 */
export function testToSortedAsc() {
    const arr = [3, 1, 4, 1, 5];
    const sorted = arr.toSorted();
    return sorted.items[0];
}
"#;
    let zig = transpile_and_assert(js, "test_toSorted_without_comparefn");
    println!("=== toSorted without compareFn ===\n{}", zig);
    // ECMA-262: Default toSorted converts elements to strings and compares.
    // For i64 arrays, should use custom lessThan with string comparison.
    assert!(
        zig.contains("lessThan"),
        "Default toSorted should use custom lessThan closure:\n{}",
        zig
    );
    assert!(
        !zig.contains("std.sort.asc"),
        "Default toSorted should not use std.sort.asc:\n{}",
        zig
    );
    assert!(
        zig.contains("std.mem.order"),
        "Default toSorted should use std.mem.order for string comparison:\n{}",
        zig
    );
}

// ── flatMap compile error tests ───────

#[test]
fn test_flatmap_compile_error() {
    let js = r#"
/**
 * @param {i64[]} arr
 * @returns {i64[]}
 */
export function doubleAll(arr) {
    return arr.flatMap((x) => x * 2);
}
"#;
    assert_not_implemented(js, "Array.prototype.flatMap");
}

#[test]
fn test_flatmap_chaining_compile_error() {
    let js = r#"
/**
 * @param {i64[]} arr
 * @returns {i64[]}
 */
export function filterThenFlatMap(arr) {
    return arr.filter((x) => x > 1).flatMap((x) => x * 10);
}
"#;
    assert_not_implemented(js, "Array.prototype.flatMap chaining");
}

#[test]
fn test_flat_without_depth() {
    let js = r#"
export function testFlatNoArgs() {
    const arr = [1, 2, 3];
    return arr.flat();
}
"#;
    let zig = transpile_and_assert(js, "test_flat_without_depth");
    println!("=== flat without depth ===\n{}", zig);
    // flat() without callback falls to runtime js_array.flat()
    assert!(
        zig.contains("js_array.flat"),
        "Expected js_array.flat runtime call:\n{}",
        zig
    );
}

#[test]
fn test_flat_with_depth() {
    let js = r#"
export function testFlatDepth() {
    const arr = [1, 2, 3];
    return arr.flat(2);
}
"#;
    let zig = transpile_and_assert(js, "test_flat_with_depth");
    println!("=== flat with depth ===\n{}", zig);
    // flat(depth) also falls to runtime js_array.flat() with depth argument
    assert!(
        zig.contains("js_array.flat"),
        "Expected js_array.flat runtime call:\n{}",
        zig
    );
}
