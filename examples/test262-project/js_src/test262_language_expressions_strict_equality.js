// test262_language_expressions_strict_equality.js
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.sameValue failed"); };
    }
}
function assert_not_same_value(actual, unexpected, message) {
    if (actual === unexpected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.notSameValue failed"); };
    }
}

export function test262_language_expressions_strict_equality() {
    assert_same_value(1 === 1, true, "1 === 1 is true");
    assert_same_value(1 === 2, false, "1 === 2 is false");
    assert_not_same_value(1, 2, "1 !== 2");
    assert_same_value(true === true, true, "true === true");
}
