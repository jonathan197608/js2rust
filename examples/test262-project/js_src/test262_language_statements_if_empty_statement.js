// test262_language_statements_if_empty_statement.js
// Source: test262/test/language/statements/if/empty-statement.js

// --- test262 harness (non-exported, anytype params per Rule 7) ---
function assert_same_value(actual, expected, message) {
    if (actual !== expected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.sameValue failed"); };
    }
}
function assert_not_same_value(actual, expected, message) {
    if (actual === expected) {
        if (message) { host_assert_fail(message); } else { host_assert_fail("assert.notSameValue failed"); };
    }
}

export function test262_language_statements_if_empty_statement() {
// Copyright 2019 Mike Pennisi.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



if(1);
}
