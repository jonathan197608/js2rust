// T-BUILTIN-02: Math.floor / ceil / round (float params naturally inferred)
function testMathFloor(x) { return Math.floor(x); }
function testMathCeil(x) { return Math.ceil(x); }
function testMathRound(x) { return Math.round(x); }
const test_floor = testMathFloor(3.7);
const test_ceil = testMathCeil(3.2);
const test_round = testMathRound(3.5);

// T-BUILTIN-05: Math.sign (re-implemented via if-else)
function testSign(x) {
    if (x > 0) return 1;
    if (x < 0) return -1;
    return 0;
}
const test_sign_pos = testSign(42); // => 1
const test_sign_neg = testSign(-5); // => -1
const test_sign_zero = testSign(0); // => 0

// T-BUILTIN-06: parseInt
function testParseInt(s) { return parseInt(s); }
const test_parse_int = testParseInt("42"); // => 42

// T-BUILTIN-02b: Math.sqrt with proper float input
function testSqrt(x) { return Math.sqrt(x); }
const test_sqrt = testSqrt(2.5);

// Smoke: Math.abs with float that has non-zero fractional
function testAbsF(x) { return Math.abs(x); }
const test_abs = testAbsF(-3.5);

// Smoke: Math.pow with float input
function testPow(b, e) { return Math.pow(b, e); }
const test_pow = testPow(2.5, 3.5);
