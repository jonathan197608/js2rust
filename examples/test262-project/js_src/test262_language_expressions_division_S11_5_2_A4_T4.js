// test262_language_expressions_division_S11_5_2_A4_T4.js
// Source: test262/test/language/expressions/division/S11.5.2_A4_T4.js

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

export function test262_language_expressions_division_S11_5_2_A4_T4() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (isNaN(Number.NEGATIVE_INFINITY / Number.NEGATIVE_INFINITY) !== true) {
  host_assert_fail('#1: -Infinity / -Infinity === Not-a-Number. Actual: ' + (-Infinity / -Infinity));
}

//CHECK#2
if (isNaN(Number.POSITIVE_INFINITY / Number.POSITIVE_INFINITY) !== true) {
  host_assert_fail('#2: Infinity / Infinity === Not-a-Number. Actual: ' + (Infinity / Infinity));
}

//CHECK#3
if (isNaN(Number.NEGATIVE_INFINITY / Number.POSITIVE_INFINITY) !== true) {
  host_assert_fail('#3: -Infinity / Infinity === Not-a-Number. Actual: ' + (-Infinity / Infinity));
}

//CHECK#4
if (isNaN(Number.POSITIVE_INFINITY / Number.NEGATIVE_INFINITY) !== true) {
  host_assert_fail('#4: Infinity / -Infinity === Not-a-Number. Actual: ' + (Infinity / -Infinity));
}
}
