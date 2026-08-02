// test262_language_expressions_logical_and.js
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.sameValue failed"); };
    }
}

export function test262_language_expressions_logical_and() {
    assert_same_value(true && true, true, "true && true === true");
    assert_same_value(true && false, false, "true && false === false");
    assert_same_value(false && true, false, "false && true === false");
    assert_same_value(false && false, false, "false && false === false");
    assert_same_value(1 && 2, 2, "1 && 2 === 2");
    assert_same_value(0 && 1, 0, "0 && 1 === 0");
}
