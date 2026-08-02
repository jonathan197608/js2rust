// test262_language_expressions_subtraction_S11_6_2_A4_T7.js
// Source: test262/test/language/expressions/subtraction/S11.6.2_A4_T7.js

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

export function test262_language_expressions_subtraction_S11_6_2_A4_T7() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (Number.MIN_VALUE - Number.MIN_VALUE !== +0) {  
  host_assert_fail('#1.1: Number.MIN_VALUE - Number.MIN_VALUE === 0. Actual: ' + (Number.MIN_VALUE - Number.MIN_VALUE));
} else {
  if (1 / (Number.MIN_VALUE - Number.MIN_VALUE) !== Number.POSITIVE_INFINITY) {
    host_assert_fail('#1.2: Number.MIN_VALUE - Number.MIN_VALUE === + 0. Actual: -0');
  }
}

//CHECK#2
if (-Number.MAX_VALUE - -Number.MAX_VALUE !== +0) {  
  host_assert_fail('#2.2: -Number.MAX_VALUE - -Number.MAX_VALUE === 0. Actual: ' + (-Number.MAX_VALUE - -Number.MAX_VALUE));
} else {
  if (1 / (-Number.MAX_VALUE - -Number.MAX_VALUE) !== Number.POSITIVE_INFINITY) {
    host_assert_fail('#2.1: -Number.MAX_VALUE - -Number.MAX_VALUE === + 0. Actual: -0');
  }
}

//CHECK#3
if (1 / Number.MAX_VALUE - 1 / Number.MAX_VALUE !== +0) {  
  host_assert_fail('#3.1: 1 / Number.MAX_VALUE - 1 / Number.MAX_VALUE === 0. Actual: ' + (1 / Number.MAX_VALUE - 1 / Number.MAX_VALUE));
} else {
  if (1 / (1 / Number.MAX_VALUE - 1 / Number.MAX_VALUE) !== Number.POSITIVE_INFINITY) {
    host_assert_fail('#3.2: 1 / Number.MAX_VALUE - 1 / Number.MAX_VALUE === + 0. Actual: -0');
  }
}
}
