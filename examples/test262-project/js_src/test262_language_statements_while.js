// test262_language_statements_while.js
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.sameValue failed"); };
    }
}

export function test262_language_statements_while() {
    var i = 0;
    var count = 0;
    while (i < 5) {
        count += 1;
        i += 1;
    }
    assert_same_value(count, 5, "while loop count === 5");
    assert_same_value(i, 5, "i === 5 after while loop");

    var j = 0;
    do {
        j += 1;
    } while (j < 3);
    assert_same_value(j, 3, "do-while j === 3");
}
