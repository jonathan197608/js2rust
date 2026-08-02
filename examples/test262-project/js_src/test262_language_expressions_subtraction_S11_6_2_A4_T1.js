// test262_language_expressions_subtraction_S11_6_2_A4_T1.js
// Source: test262/test/language/expressions/subtraction/S11.6.2_A4_T1.js

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

export function test262_language_expressions_subtraction_S11_6_2_A4_T1() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (isNaN(Number.NaN - 1) !== true ) {
  host_assert_fail('#1: NaN - 1 === Not-a-Number. Actual: ' + (NaN - 1));
}

//CHECK#2
if (isNaN(1 - Number.NaN) !== true ) {
  host_assert_fail('#2: 1 - NaN === Not-a-Number. Actual: ' + (1 - NaN));
}

//CHECK#3
if (isNaN(Number.NaN - Number.POSITIVE_INFINITY) !== true ) {
  host_assert_fail('#3: NaN - Infinity === Not-a-Number. Actual: ' + (NaN - Infinity));
}

//CHECK#4
if (isNaN(Number.POSITIVE_INFINITY - Number.NaN) !== true ) {
  host_assert_fail('#4: Infinity - NaN === Not-a-Number. Actual: ' + (Infinity - NaN));
}

//CHECK#5
if (isNaN(Number.NaN - Number.NEGATIVE_INFINITY) !== true ) {
  host_assert_fail('#5: NaN - Infinity === Not-a-Number. Actual: ' + (NaN - Infinity));
}

//CHECK#6
if (isNaN(Number.NEGATIVE_INFINITY - Number.NaN) !== true ) {
  host_assert_fail('#6: Infinity - NaN === Not-a-Number. Actual: ' + (Infinity - NaN));
}
}
