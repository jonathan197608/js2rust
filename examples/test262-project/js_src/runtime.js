// --- test262 runtime: shared assert helpers ---
// Non-exported functions → Zig anytype params (Rule 7)
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message !== undefined) { host_assert_fail(message); }
        else { host_assert_fail("assert.sameValue failed"); }
    }
}
function assert_not_same_value(actual, expected, message) {
    if (actual === expected) {
        if (message !== undefined) { host_assert_fail(message); }
        else { host_assert_fail("assert.notSameValue failed"); }
    }
}
