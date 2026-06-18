// T-INF-01: Layer 1 — literal type inference
function intLiteral() { return 42; }
function floatLiteral() { return 3.14; }
function boolLiteral() { return true; }

const test_int_lit = intLiteral(); // => 42
const test_float_lit = floatLiteral(); // => 3.14
const test_bool_lit = boolLiteral(); // => true

// T-INF-03: binary expression type inference
function addInts(a, b) { return a + b; }
function compare(a, b) { return a > b; }

const test_add_ints = addInts(3, 5); // => 8
const test_compare = compare(5, 3); // => true

// T-INF-04: const + function call tracking (Rule 2.2)
function getNumber() { return 42; }
function wrapper() {
    const result = getNumber();
    return result + 1;
}
const test_wrapper = wrapper(); // => 43

// T-INF-08: multi-branch return type
function multiReturn(x) {
    if (x > 0) { return 1; }
    if (x < 0) { return -1; }
    return 0;
}
const test_multi_ret = multiReturn(5); // => 1

// T-INF-13: default parameter — caller must pass all args (Zig limitation)
function withDefault(x, y) {
    return x + y;
}
const test_default = withDefault(5, 10); // => 15
