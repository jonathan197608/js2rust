// test262_language_expressions_multiplication.js
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.sameValue failed"); };
    }
}

export function test262_language_expressions_multiplication() {
    assert_same_value(2 * 3, 6, "2 * 3 === 6");
    assert_same_value(-2 * 3, -6, "-2 * 3 === -6");
    assert_same_value(0 * 5, 0, "0 * 5 === 0");
    assert_same_value(10 * 10, 100, "10 * 10 === 100");
}
