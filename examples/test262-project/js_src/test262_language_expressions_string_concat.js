// test262_language_expressions_string_concat.js
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.sameValue failed"); };
    }
}

export function test262_language_expressions_string_concat() {
    assert_same_value("a" + "b", "ab", '"a" + "b" === "ab"');
    assert_same_value("" + "", "", '"" + "" === ""');
    assert_same_value("1" + "2", "12", '"1" + "2" === "12"');
}
