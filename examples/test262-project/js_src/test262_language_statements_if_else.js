// test262_language_statements_if_else.js
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.sameValue failed"); };
    }
}

export function test262_language_statements_if_else() {
    var x = 5;
    if (x > 3) {
        assert_same_value(x, 5, "x should be 5");
    } else {
        host_assert_fail("should not reach else branch");
    }

    if (x < 0) {
        host_assert_fail("should not reach if branch");
    } else if (x === 5) {
        assert_same_value(x, 5, "x should be 5 in else-if");
    } else {
        host_assert_fail("should not reach final else");
    }
}
