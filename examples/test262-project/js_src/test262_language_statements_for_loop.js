// test262_language_statements_for_loop.js
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.sameValue failed"); };
    }
}

export function test262_language_statements_for_loop() {
    var sum = 0;
    for (var i = 0; i < 10; i++) {
        sum += i;
    }
    assert_same_value(sum, 45, "sum of 0..9 === 45");

    var product = 1;
    for (var j = 1; j <= 5; j++) {
        product *= j;
    }
    assert_same_value(product, 120, "5! === 120");
}
