// ZigIR dual-track validation tests.
// For each JS input, compare Codegen output with Lowerer+Emitter output.

use super::common::parse_and_transpile;
use crate::zigir::emit::Emitter;
use crate::zigir::lower::Lowerer;

/// Run both Codegen and Lowerer+Emitter on the same JS input and return both outputs.
fn dual_track(js: &str) -> (String, String) {
    // Step 1: Run Codegen (the original path)
    let codegen_result = parse_and_transpile(js, None).unwrap();
    let codegen_output = codegen_result.zig_code;

    // Step 2: Run Lowerer + Emitter (needs same inputs)
    // We need to re-parse and re-infer because Codegen consumed the originals.
    let alloc = oxc_allocator::Allocator::default();
    let program = crate::parser::parse(&alloc, js);

    // Re-run type inference
    let (typedefs, type_annotations, return_types, param_types) =
        crate::jsdoc::extract_all_jsdoc(js);
    let jsdoc_data = crate::types::JSDocData {
        typedefs,
        type_annotations,
        return_types,
        param_types,
    };
    let mut inferrer = crate::infer::TypeInferrer::new();
    inferrer.set_jsdoc_data(jsdoc_data.clone());
    let type_info = inferrer.infer_all(&program, None);

    let mut lowerer = Lowerer::new(
        type_info,
        jsdoc_data,
        None,
        std::collections::HashSet::new(),
        js.to_string(),
    );
    let ir_module = lowerer.lower(&program);
    let emitter_output = Emitter::emit_module(&ir_module);

    (codegen_output, emitter_output)
}

/// Compare two outputs and return the first `max_diff` differing line pairs.
fn diff_outputs(codegen: &str, emitter: &str, max_diff: usize) -> Vec<(usize, String, String)> {
    let mut diffs = Vec::new();
    let c_lines: Vec<&str> = codegen.lines().collect();
    let e_lines: Vec<&str> = emitter.lines().collect();
    let max_len = c_lines.len().max(e_lines.len());
    for i in 0..max_len {
        let c = c_lines.get(i).copied().unwrap_or("");
        let e = e_lines.get(i).copied().unwrap_or("");
        if c != e {
            diffs.push((i + 1, c.to_string(), e.to_string()));
            if diffs.len() >= max_diff {
                break;
            }
        }
    }
    diffs
}

// ═══════════════════════════════════════════════════════
//  Basic function tests
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_simple_function() {
    let js = r#"
/**
 * @returns {number}
 */
function add(a, b) {
    return a + b;
}
"#;
    let (codegen, emitter) = dual_track(js);
    if codegen != emitter {
        let diffs = diff_outputs(&codegen, &emitter, 5);
        eprintln!(
            "[dual-track] simple_function: {} differing lines",
            diffs.len()
        );
        for (line_no, c, e) in &diffs {
            eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
        }
    }
    // For now, just log the diff — don't assert equality yet.
    // Will tighten once the Emitter is calibrated.
    println!(
        "[dual-track] simple_function: codegen={} lines, emitter={} lines",
        codegen.lines().count(),
        emitter.lines().count()
    );
}

#[test]
fn test_dual_track_variable_declarations() {
    let js = r#"
/**
 * @type {number}
 */
const x = 42;

/**
 * @type {string}
 */
const name = "hello";
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] variable_declarations: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

#[test]
fn test_dual_track_if_else() {
    let js = r#"
/**
 * @param {number} x
 * @returns {number}
 */
function abs(x) {
    if (x < 0) {
        return -x;
    } else {
        return x;
    }
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] if_else: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

#[test]
fn test_dual_track_while_loop() {
    let js = r#"
/**
 * @param {number} n
 * @returns {number}
 */
function sumTo(n) {
    let total = 0;
    let i = 0;
    while (i < n) {
        total = total + i;
        i = i + 1;
    }
    return total;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] while_loop: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    if !diffs.is_empty() {
        for (line_no, c, e) in &diffs {
            eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
        }
    }
}

#[test]
fn test_dual_track_typedef() {
    let js = r#"
/**
 * @typedef {Object} Point
 * @property {number} x
 * @property {number} y
 */

/**
 * @param {Point} p
 * @returns {number}
 */
function getX(p) {
    return p.x;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] typedef: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  For loop / assignment / update
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_for_loop() {
    let js = r#"
/**
 * @param {number} n
 * @returns {number}
 */
function sumTo(n) {
    let total = 0;
    for (let i = 0; i < n; i += 1) {
        total = total + i;
    }
    return total;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] for_loop: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

#[test]
fn test_dual_track_assignment_ops() {
    let js = r#"
/**
 * @param {number} x
 * @returns {number}
 */
function addAssign(x) {
    let y = x;
    y += 1;
    y -= 2;
    y *= 3;
    return y;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] assignment_ops: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Ternary / logical / comparison
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_ternary() {
    let js = r#"
/**
 * @param {number} x
 * @returns {number}
 */
function abs(x) {
    return x < 0 ? -x : x;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] ternary: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

#[test]
fn test_dual_track_logical_ops() {
    let js = r#"
/**
 * @param {number} x
 * @param {number} y
 * @returns {number}
 */
function logic(x, y) {
    if (x > 0 && y > 0) {
        return 1;
    }
    if (x < 0 || y < 0) {
        return -1;
    }
    return 0;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] logical_ops: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  String literals / template literals
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_string_concat() {
    let js = r#"
/**
 * @param {string} name
 * @returns {string}
 */
function greet(name) {
    return "Hello, " + name + "!";
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] string_concat: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Nested if / switch
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_switch() {
    let js = r#"
/**
 * @param {number} x
 * @returns {number}
 */
function classify(x) {
    switch (x) {
        case 0:
            return 1;
        case 1:
            return 2;
        default:
            return 3;
    }
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] switch: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Try-catch / throw
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_try_catch() {
    let js = r#"
/**
 * @returns {number}
 */
function mayFail() {
    try {
        return 42;
    } catch (e) {
        return -1;
    }
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] try_catch: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Arrow functions / closures
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_arrow_simple() {
    let js = r#"
/**
 * @type {function(number): number}
 */
const double = (x) => x * 2;
"#;
    let (codegen, emitter) = dual_track(js);
    // Debug: print both outputs line by line
    eprintln!("=== CODEGEN ===");
    for (i, line) in codegen.lines().enumerate() {
        eprintln!("{:3}: [{}]", i + 1, line);
    }
    eprintln!("=== EMITTER ===");
    for (i, line) in emitter.lines().enumerate() {
        eprintln!("{:3}: [{}]", i + 1, line);
    }
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] arrow_simple: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

#[test]
fn test_dual_track_closure_capture() {
    let js = r#"
/**
 * @param {number} x
 * @returns {function(): number}
 */
function makeCounter(x) {
    let count = x;
    return function() {
        count += 1;
        return count;
    };
}
"#;
    let (codegen, emitter) = dual_track(js);
    // Debug: print both outputs
    eprintln!("=== CODEGEN ===");
    for (i, line) in codegen.lines().enumerate() {
        eprintln!("{:3}: [{}]", i + 1, line);
    }
    eprintln!("=== EMITTER ===");
    for (i, line) in emitter.lines().enumerate() {
        eprintln!("{:3}: [{}]", i + 1, line);
    }
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] closure_capture: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Multiple functions (inter-decl spacing)
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_multiple_functions() {
    let js = r#"
/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
function add(a, b) {
    return a + b;
}

/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
function mul(a, b) {
    return a * b;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] multiple_functions: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Math builtins
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_math_builtin() {
    let js = r#"
/**
 * @param {number} x
 * @returns {number}
 */
function sqrtOf(x) {
    return Math.sqrt(x);
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] math_builtin: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Do-while loop
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_do_while() {
    let js = r#"
/**
 * @param {number} n
 * @returns {number}
 */
function countDown(n) {
    let total = 0;
    do {
        total += 1;
        n -= 1;
    } while (n > 0);
    return total;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] do_while: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Update expressions (i++, i--)
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_update_expr() {
    let js = r#"
/**
 * @param {number} x
 * @returns {number}
 */
function inc(x) {
    x += 1;
    x++;
    return x;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] update_expr: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Unary not
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_unary_not() {
    let js = r#"
/**
 * @param {boolean} flag
 * @returns {boolean}
 */
function negate(flag) {
    return !flag;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] unary_not: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Unary negate
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_unary_negate() {
    let js = r#"
/**
 * @param {number} x
 * @returns {number}
 */
function neg(x) {
    return -x;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] unary_negate: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Nested if / else-if
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_nested_if() {
    let js = r#"
/**
 * @param {number} x
 * @returns {number}
 */
function classify(x) {
    if (x > 0) {
        return 1;
    } else if (x < 0) {
        return -1;
    } else {
        return 0;
    }
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] nested_if: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Break / continue in loops
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_break_continue() {
    let js = r#"
/**
 * @param {number} n
 * @returns {number}
 */
function sumOdds(n) {
    let total = 0;
    for (let i = 0; i < n; i += 1) {
        if (i === 5) {
            break;
        }
        if (i % 2 === 0) {
            continue;
        }
        total += i;
    }
    return total;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] break_continue: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Switch with break
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_switch_break() {
    let js = r#"
/**
 * @param {number} x
 * @returns {number}
 */
function switchBreak(x) {
    let result = 0;
    switch (x) {
        case 1:
            result = 10;
            break;
        case 2:
            result = 20;
            break;
        default:
            result = 30;
            break;
    }
    return result;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] switch_break: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Array literal
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_array_literal() {
    let js = r#"
/**
 * @returns {number[]}
 */
function getNums() {
    const nums = [1, 2, 3];
    return nums;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] array_literal: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Object literal
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_object_literal() {
    let js = r#"
/**
 * @typedef {Object} Point
 * @property {number} x
 * @property {number} y
 */

/**
 * @returns {Point}
 */
function makePoint() {
    const p = { x: 1, y: 2 };
    return p;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] object_literal: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Try-catch with throw (Case A)
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_try_catch_throw() {
    let js = r#"
/**
 * @param {boolean} shouldThrow
 * @returns {number}
 */
function tryThrow(shouldThrow) {
    try {
        if (shouldThrow) {
            throw "error";
        }
        return 1;
    } catch (e) {
        return -1;
    }
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 15);
    println!(
        "[dual-track] try_catch_throw: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Try-finally (B1/B2)
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_try_finally() {
    let js = r#"
/**
 * @returns {number}
 */
function tryFinally() {
    let x = 0;
    try {
        x = 1;
    } finally {
        x = 2;
    }
    return x;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] try_finally: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Multi-param arrow
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_arrow_multi_param() {
    let js = r#"
/**
 * @type {function(number, number): number}
 */
const add = (a, b) => a + b;
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] arrow_multi_param: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Strict equality / inequality
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_equality() {
    let js = r#"
/**
 * @param {number} x
 * @param {number} y
 * @returns {boolean}
 */
function isEqual(x, y) {
    if (x === y) {
        return true;
    }
    if (x !== y) {
        return false;
    }
    return false;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] equality: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Null literal
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_null() {
    let js = r#"
/**
 * @returns {null}
 */
function getNull() {
    return null;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] null: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Remainder operator
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_remainder() {
    let js = r#"
/**
 * @param {number} x
 * @param {number} y
 * @returns {number}
 */
function mod(x, y) {
    return x % y;
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] remainder: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Arrow block body
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_arrow_block_body() {
    let js = r#"
/**
 * @param {number} x
 * @type {function(number): number}
 */
const square = (x) => {
    return x * x;
};
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] arrow_block_body: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}

// ═══════════════════════════════════════════════════════
//  Multi-capture closure
// ═══════════════════════════════════════════════════════

#[test]
fn test_dual_track_multi_capture_closure() {
    let js = r#"
/**
 * @param {number} a
 * @param {number} b
 * @returns {function(): number}
 */
function makeAdder(a, b) {
    return function() {
        return a + b;
    };
}
"#;
    let (codegen, emitter) = dual_track(js);
    let diffs = diff_outputs(&codegen, &emitter, 10);
    println!(
        "[dual-track] multi_capture_closure: codegen={} lines, emitter={} lines, {} diffs",
        codegen.lines().count(),
        emitter.lines().count(),
        diffs.len()
    );
    for (line_no, c, e) in &diffs {
        eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
    }
}
