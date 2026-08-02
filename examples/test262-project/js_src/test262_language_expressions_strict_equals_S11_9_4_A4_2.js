// test262_language_expressions_strict_equals_S11_9_4_A4_2.js
// Source: test262/test/language/expressions/strict-equals/S11.9.4_A4.2.js

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

export function test262_language_expressions_strict_equals_S11_9_4_A4_2() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (!(+0 === -0)) {
  host_assert_fail('#1: +0 === -0');
}

//CHECK#2
if (!(-0 === +0)) {
  host_assert_fail('#2: -0 === +0');
}
}
