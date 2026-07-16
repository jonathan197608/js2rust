// Logical assignment operators &&= / ||= on i64, and sequence expression.
// Expanded to Assign + Logical in the Lowerer (operators.rs),
// reusing the Logical emitter's js_runtime.isTruthy() (anytype) and
// JsAny.from() wrapping for type-safe branch emission.

// ── &&= on i64 ──

/** @returns {i64} */
export function testAndAssignTruthy() {
    let a = 5;
    a &&= 10;  // a is truthy (5 != 0), so a = 10
    return a;  // expected 10
}

/** @returns {i64} */
export function testAndAssignFalsy() {
    let a = 0;
    a &&= 10;  // a is falsy (0), so a stays 0
    return a;  // expected 0
}

// ── ||= on i64 ──

/** @returns {i64} */
export function testOrAssignFalsy() {
    let a = 0;
    a ||= 10;  // a is falsy (0), so a = 10
    return a;  // expected 10
}

/** @returns {i64} */
export function testOrAssignTruthy() {
    let a = 5;
    a ||= 10;  // a is truthy (5), so a stays 5
    return a;  // expected 5
}

// ── Sequence expression (comma operator) ──

/** @returns {i64} */
export function testSequenceExpr() {
    let r = (1, 2, 3);  // JS comma: evaluate all, return last → r = 3
    return r;  // expected 3
}
