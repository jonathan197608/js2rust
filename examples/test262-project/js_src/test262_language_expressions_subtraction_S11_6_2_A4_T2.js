// test262_language_expressions_subtraction_S11_6_2_A4_T2.js
// Source: test262/test/language/expressions/subtraction/S11.6.2_A4_T2.js

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

export function test262_language_expressions_subtraction_S11_6_2_A4_T2() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (Number.POSITIVE_INFINITY - Number.NEGATIVE_INFINITY !== Number.POSITIVE_INFINITY ) {
  host_assert_fail('#1: Infinity - -Infinity === Infinity. Actual: ' + (Infinity - -Infinity));
}

//CHECK#2
if (Number.NEGATIVE_INFINITY - Number.POSITIVE_INFINITY !== Number.NEGATIVE_INFINITY ) {
  host_assert_fail('#2: -Infinity - Infinity === -Infinity. Actual: ' + (-Infinity - Infinity));
}
}
