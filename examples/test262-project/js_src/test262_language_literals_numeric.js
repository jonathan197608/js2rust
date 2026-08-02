// test262_language_literals_numeric.js
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.sameValue failed"); };
    }
}

export function test262_language_literals_numeric() {
    assert_same_value(42, 42, "integer literal");
    assert_same_value(-1, -1, "negative integer literal");
    assert_same_value(0, 0, "zero");
    assert_same_value(0.5 + 0.5, 1, "0.5 + 0.5 === 1");
}
