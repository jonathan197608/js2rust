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

/// Macro to reduce boilerplate for dual-track tests.
/// Usage: `dual_track_test!(test_name, "JS code");`
/// The test name after `test_dual_track_` is used as the label in log output.
macro_rules! dual_track_test {
    ($name:ident, $js:expr) => {
        #[test]
        fn $name() {
            let label = stringify!($name)
                .strip_prefix("test_dual_track_")
                .unwrap_or(stringify!($name));
            let (codegen, emitter) = dual_track($js);
            if codegen != emitter {
                let diffs = diff_outputs(&codegen, &emitter, 10);
                eprintln!("[dual-track] {}: {} differing lines", label, diffs.len());
                for (line_no, c, e) in &diffs {
                    eprintln!("  line {}: codegen='{}' emitter='{}'", line_no, c, e);
                }
            }
            println!(
                "[dual-track] {}: codegen={} lines, emitter={} lines",
                label,
                codegen.lines().count(),
                emitter.lines().count()
            );
        }
    };
}

// ═══════════════════════════════════════════════════════
//  Basic function tests
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_simple_function,
    r#"
/**
 * @returns {number}
 */
function add(a, b) {
    return a + b;
}
"#
);

dual_track_test!(
    test_dual_track_variable_declarations,
    r#"
/**
 * @type {number}
 */
const x = 42;

/**
 * @type {string}
 */
const name = "hello";
"#
);

dual_track_test!(
    test_dual_track_if_else,
    r#"
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
"#
);

dual_track_test!(
    test_dual_track_while_loop,
    r#"
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
"#
);

dual_track_test!(
    test_dual_track_typedef,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  For loop / assignment / update
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_for_loop,
    r#"
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
"#
);

dual_track_test!(
    test_dual_track_assignment_ops,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  Ternary / logical / comparison
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_ternary,
    r#"
/**
 * @param {number} x
 * @returns {number}
 */
function abs(x) {
    return x < 0 ? -x : x;
}
"#
);

dual_track_test!(
    test_dual_track_logical_ops,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  String literals / template literals
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_string_concat,
    r#"
/**
 * @param {string} name
 * @returns {string}
 */
function greet(name) {
    return "Hello, " + name + "!";
}
"#
);

// ═══════════════════════════════════════════════════════
//  Nested if / switch
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_switch,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  Try-catch / throw
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_try_catch,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  Arrow functions / closures
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_arrow_simple,
    r#"
/**
 * @type {function(number): number}
 */
const double = (x) => x * 2;
"#
);

dual_track_test!(
    test_dual_track_closure_capture,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  Multiple functions (inter-decl spacing)
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_multiple_functions,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  Math builtins
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_math_builtin,
    r#"
/**
 * @param {number} x
 * @returns {number}
 */
function sqrtOf(x) {
    return Math.sqrt(x);
}
"#
);

// ═══════════════════════════════════════════════════════
//  Do-while loop
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_do_while,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  Update expressions (i++, i--)
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_update_expr,
    r#"
/**
 * @param {number} x
 * @returns {number}
 */
function inc(x) {
    x += 1;
    x++;
    return x;
}
"#
);

// ═══════════════════════════════════════════════════════
//  Unary not / negate
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_unary_not,
    r#"
/**
 * @param {boolean} flag
 * @returns {boolean}
 */
function negate(flag) {
    return !flag;
}
"#
);

dual_track_test!(
    test_dual_track_unary_negate,
    r#"
/**
 * @param {number} x
 * @returns {number}
 */
function neg(x) {
    return -x;
}
"#
);

// ═══════════════════════════════════════════════════════
//  Nested if / else-if
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_nested_if,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  Break / continue in loops
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_break_continue,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  Switch with break
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_switch_break,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  Array literal
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_array_literal,
    r#"
/**
 * @returns {number[]}
 */
function getNums() {
    const nums = [1, 2, 3];
    return nums;
}
"#
);

// ═══════════════════════════════════════════════════════
//  Object literal
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_object_literal,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  Try-catch with throw (Case A)
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_try_catch_throw,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  Try-finally (B1/B2)
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_try_finally,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  Multi-param arrow
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_arrow_multi_param,
    r#"
/**
 * @type {function(number, number): number}
 */
const add = (a, b) => a + b;
"#
);

// ═══════════════════════════════════════════════════════
//  Strict equality / inequality
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_equality,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  Null literal
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_null,
    r#"
/**
 * @returns {null}
 */
function getNull() {
    return null;
}
"#
);

// ═══════════════════════════════════════════════════════
//  Remainder operator
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_remainder,
    r#"
/**
 * @param {number} x
 * @param {number} y
 * @returns {number}
 */
function mod(x, y) {
    return x % y;
}
"#
);

// ═══════════════════════════════════════════════════════
//  Arrow block body
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_arrow_block_body,
    r#"
/**
 * @param {number} x
 * @type {function(number): number}
 */
const square = (x) => {
    return x * x;
};
"#
);

// ═══════════════════════════════════════════════════════
//  Multi-capture closure
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_multi_capture_closure,
    r#"
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
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: typeof operator
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_typeof,
    r#"
/**
 * @param {number} x
 * @returns {string}
 */
function checkType(x) {
    return typeof x;
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: void operator
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_void,
    r#"
/**
 * @returns {undefined}
 */
function retUndef() {
    return void 0;
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: template literals
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_template_literal,
    r#"
/**
 * @param {string} name
 * @returns {string}
 */
function greet(name) {
    return `Hello, ${name}!`;
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: console.log
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_console_log,
    r#"
/**
 * @param {number} x
 * @returns {number}
 */
function logAndReturn(x) {
    console.log(x);
    return x;
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: for-of loop (array iteration)
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_for_of,
    r#"
/**
 * @param {number[]} nums
 * @returns {number}
 */
function sumArray(nums) {
    let total = 0;
    for (const n of nums) {
        total += n;
    }
    return total;
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: for-in loop (object keys)
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_for_in,
    r#"
/**
 * @typedef {Object} Config
 * @property {number} verbose
 * @property {number} timeout
 */

/**
 * @param {Config} cfg
 * @returns {number}
 */
function countKeys(cfg) {
    let count = 0;
    for (const key in cfg) {
        count += 1;
    }
    return count;
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: new Date()
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_new_date,
    r#"
/**
 * @returns {Date}
 */
function now() {
    return new Date();
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: computed member access (obj[key])
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_computed_member,
    r#"
/**
 * @typedef {Object} Dict
 * @property {number} a
 * @property {number} b
 */

/**
 * @param {Dict} obj
 * @param {string} key
 * @returns {number}
 */
function getByKey(obj, key) {
    return obj[key];
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: class with this
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_class_this,
    r#"
/**
 * @typedef {Object} Counter
 * @property {number} count
 */

/**
 * @returns {Counter}
 */
function makeCounter() {
    class Counter {
        constructor() {
            this.count = 0;
        }
        increment() {
            this.count += 1;
        }
    }
    const c = new Counter();
    c.increment();
    return c;
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: labeled break in nested loops
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_labeled_break,
    r#"
/**
 * @param {number} n
 * @returns {number}
 */
function nestedBreak(n) {
    let result = 0;
    outer: for (let i = 0; i < n; i += 1) {
        for (let j = 0; j < n; j += 1) {
            if (j === 3) {
                break outer;
            }
            result += 1;
        }
    }
    return result;
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: JSON.parse
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_json_parse,
    r#"
/**
 * @param {string} json
 * @returns {Object}
 */
function parseJson(json) {
    const data = JSON.parse(json);
    return data;
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: Math.max / Math.min
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_math_min_max,
    r#"
/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
function clamp(a, b) {
    const lo = Math.min(a, b);
    const hi = Math.max(a, b);
    return hi - lo;
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: Arrow with closure capture
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_arrow_closure_capture,
    r#"
/**
 * @param {number} base
 * @returns {function(number): number}
 */
function makeAdder(base) {
    return (x) => base + x;
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: Bitwise operators
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_bitwise_ops,
    r#"
/**
 * @param {number} a
 * @param {number} b
 * @returns {number}
 */
function bitOps(a, b) {
    let x = a & b;
    x = x | 1;
    x = x ^ 2;
    x = x << 1;
    return x;
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: String.length and String methods
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_string_methods,
    r#"
/**
 * @param {string} s
 * @returns {number}
 */
function strLen(s) {
    return s.length;
}
"#
);

// ═══════════════════════════════════════════════════════
//  NEW: Multiple Math builtins
// ═══════════════════════════════════════════════════════

dual_track_test!(
    test_dual_track_math_floor_ceil,
    r#"
/**
 * @param {number} x
 * @returns {number}
 */
function roundOps(x) {
    const a = Math.floor(x);
    const b = Math.ceil(x);
    const c = Math.abs(x);
    return a + b + c;
}
"#
);
