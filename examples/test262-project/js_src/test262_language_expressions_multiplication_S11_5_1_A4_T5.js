// test262_language_expressions_multiplication_S11_5_1_A4_T5.js
// Source: test262/test/language/expressions/multiplication/S11.5.1_A4_T5.js

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

export function test262_language_expressions_multiplication_S11_5_1_A4_T5() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (Number.NEGATIVE_INFINITY * -1 !== Number.POSITIVE_INFINITY) {
  host_assert_fail('#1: -Infinity * -1 === Infinity. Actual: ' + (-Infinity * -1));
}

//CHECK#2
if (-1 * Number.NEGATIVE_INFINITY !== Number.POSITIVE_INFINITY) {
  host_assert_fail('#2: -1 * -Infinity === Infinity. Actual: ' + (-1 * -Infinity));
}

//CHECK#3
if (Number.POSITIVE_INFINITY * -1 !== Number.NEGATIVE_INFINITY) {
  host_assert_fail('#3: Infinity * -1 === -Infinity. Actual: ' + (Infinity * -1));
}

//CHECK#4
if (-1 * Number.POSITIVE_INFINITY !== Number.NEGATIVE_INFINITY) {
  host_assert_fail('#4: -1 * Infinity === -Infinity. Actual: ' + (-1 * Infinity));
}  

//CHECK#5
if (Number.POSITIVE_INFINITY * Number.MAX_VALUE !== Number.POSITIVE_INFINITY) {
  host_assert_fail('#5: Infinity * Number.MAX_VALUE === Infinity. Actual: ' + (Infinity * Number.MAX_VALUE));
}

//CHECK#6
if (Number.POSITIVE_INFINITY * Number.MAX_VALUE !== Number.MAX_VALUE * Number.POSITIVE_INFINITY) {
  host_assert_fail('#6: Infinity * Number.MAX_VALUE === Number.MAX_VALUE * Infinity. Actual: ' + (Infinity * Number.MAX_VALUE));
}

//CHECK#7
if (Number.NEGATIVE_INFINITY * Number.MIN_VALUE !== Number.NEGATIVE_INFINITY) {
  host_assert_fail('#7: -Infinity * Number.MIN_VALUE === -Infinity. Actual: ' + (-Infinity * Number.MIN_VALUE));
}

//CHECK#8
if (Number.NEGATIVE_INFINITY * Number.MIN_VALUE !== Number.MIN_VALUE * Number.NEGATIVE_INFINITY) {
  host_assert_fail('#8: -Infinity * Number.MIN_VALUE === Number.MIN_VALUE * -Infinity. Actual: ' + (-Infinity * Number.MIN_VALUE));
}
}
