// T-STMT-08: switch-case
function switchMulti(x) {
    switch (x) {
        case 1: return 10;
        case 2: return 20;
        case 3: return 30;
        default: return 0;
    }
}
const test_sw1 = switchMulti(2); // => 20
const test_sw_def = switchMulti(99); // => 0

// T-FN-01: basic arithmetic chaining
function chain(a, b, c) {
    return a + b + c;
}
const test_chain = chain(1, 2, 3); // => 6

// T-STMT-09b: nested if-else
function clamp(x, lo, hi) {
    if (x < lo) { return lo; }
    if (x > hi) { return hi; }
    return x;
}
const test_clamp_low = clamp(-5, 0, 100); // => 0
const test_clamp_mid = clamp(50, 0, 100); // => 50
const test_clamp_hi = clamp(200, 0, 100); // => 100

// T-FN-08: multi-return branches — if-else
function sign(x) {
    if (x > 0) { return 1; }
    if (x < 0) { return -1; }
    return 0;
}
const test_sign_pos = sign(42); // => 1
const test_sign_neg = sign(-7); // => -1
const test_sign_zero = sign(0); // => 0

// T-EXPR-04b: nested ternary
function abs(x) { return x >= 0 ? x : -x; }
const test_abs_pos = abs(5); // => 5
const test_abs_neg = abs(-5); // => 5

// T-FN-12b: min/max via comparison
function min(a, b) { return a < b ? a : b; }
function max(a, b) { return a > b ? a : b; }
const test_min = min(3, 7); // => 3
const test_max = max(3, 7); // => 7
