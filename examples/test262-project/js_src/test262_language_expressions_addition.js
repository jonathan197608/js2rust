// test262_language_expressions_addition.js
// Source: test262/test/language/expressions/addition

// --- harness (non-exported, anytype params) ---
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.sameValue failed"); };
    }
}

// --- test262 original code ---
export function test262_language_expressions_addition() {
    assert_same_value(1 + 2, 3, "1 + 2 === 3");
    assert_same_value(-1 + 1, 0, "-1 + 1 === 0");
    assert_same_value(0 + 0, 0, "0 + 0 === 0");
    assert_same_value(100 + 200, 300, "100 + 200 === 300");
}
