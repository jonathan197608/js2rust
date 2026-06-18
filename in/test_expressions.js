// T-EXPR-01: numeric literals (hex, negative)
function hexNum() { return 0xFF; }
function negNum() { return -42; }

const test_hex = hexNum(); // => 255
const test_neg = negNum(); // => -42

// T-EXPR-09: object literal → anonymous struct
function makeObj() {
    const obj = { x: 1, y: 2 };
    return obj.x + obj.y;
}
const test_obj = makeObj(); // => 3

// T-EXPR-11: array literal + index access
function arrayLiteral() {
    const arr = [10, 20, 30];
    return arr[1];
}
const test_arr_lit = arrayLiteral(); // => 20

// T-EXPR-21: parenthesized expression
function parenExpr(a, b) {
    return (a + b) * 2;
}
const test_paren = parenExpr(3, 4); // => 14

// T-EXPR-23: strict equality operators
function strictEq(a, b) { return a === b; }
function strictNeq(a, b) { return a !== b; }
const test_eq_true = strictEq(5, 5); // => true
const test_eq_false = strictEq(5, 3); // => false
const test_neq = strictNeq(5, 3); // => true

// T-EXPR-24: comparison operators
function lt(a, b) { return a < b; }
function le(a, b) { return a <= b; }
function gt(a, b) { return a > b; }
function ge(a, b) { return a >= b; }

const test_lt = lt(3, 5); // => true
const test_le = le(5, 5); // => true
const test_gt = gt(5, 3); // => true
const test_ge = ge(3, 5); // => false

// T-EXPR-04: ternary operator
function ternary(x) { return x > 0 ? 1 : -1; }
const test_ternary_pos = ternary(5); // => 1
const test_ternary_neg = ternary(-3); // => -1

// T-EXPR-25: unary operators (-, !)
function unaryNeg(x) { return -x; }
const test_uneg = unaryNeg(5); // => -5

// T-FN-01: basic function declaration
function simpleAdd(a, b) { return a + b; }
const test_fn_basic = simpleAdd(10, 20); // => 30

// T-STMT-09: arrow function assignment
const square = (x) => { return x * x; };
const test_arrow_var = square(5); // => 25

// T-STMT-11: multi-variable declaration
function multiDecl() {
    const a = 1, b = 2, c = 3;
    return a + b + c;
}
const test_multi_decl = multiDecl(); // => 6
