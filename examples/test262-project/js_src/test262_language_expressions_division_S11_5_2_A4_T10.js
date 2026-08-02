// test262_language_expressions_division_S11_5_2_A4_T10.js
// Source: test262/test/language/expressions/division/S11.5.2_A4_T10.js

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

export function test262_language_expressions_division_S11_5_2_A4_T10() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (Number.MIN_VALUE / 2.1 !== 0) {
  host_assert_fail('#1: Number.MIN_VALUE / 2.1 === 0. Actual: ' + (Number.MIN_VALUE / 2.1));
}

//CHECK#2
if (Number.MIN_VALUE / -2.1 !== -0) {
  host_assert_fail('#2.1: Number.MIN_VALUE / -2.1 === 0. Actual: ' + (Number.MIN_VALUE / -2.1));
} else {
  if (1 / (Number.MIN_VALUE / -2.1) !== Number.NEGATIVE_INFINITY) {
    host_assert_fail('#2.2: Number.MIN_VALUE / -2.1 === -0. Actual: +0');
  }
}

//CHECK#3
if (Number.MIN_VALUE / 2.0 !== 0) {
  host_assert_fail('#3: Number.MIN_VALUE / 2.0 === 0. Actual: ' + (Number.MIN_VALUE / 2.0));
}

//CHECK#4
if (Number.MIN_VALUE / -2.0 !== -0) {
  host_assert_fail('#4.1: Number.MIN_VALUE / -2.0 === -0. Actual: ' + (Number.MIN_VALUE / -2.0));
} else {
  if (1 / (Number.MIN_VALUE / -2.0) !== Number.NEGATIVE_INFINITY) {
    host_assert_fail('#4.2: Number.MIN_VALUE / -2.0 === -0. Actual: +0');
  }
}

//CHECK#5
if (Number.MIN_VALUE / 1.9 !== Number.MIN_VALUE) {
  host_assert_fail('#5: Number.MIN_VALUE / 1.9 === Number.MIN_VALUE. Actual: ' + (Number.MIN_VALUE / 1.9));
}

//CHECK#6
if (Number.MIN_VALUE / -1.9 !== -Number.MIN_VALUE) {
  host_assert_fail('#6: Number.MIN_VALUE / -1.9 === -Number.MIN_VALUE. Actual: ' + (Number.MIN_VALUE / -1.9));
}

//CHECK#7
if (Number.MIN_VALUE / 1.1 !== Number.MIN_VALUE) {
  host_assert_fail('#7: Number.MIN_VALUE / 1.1 === Number.MIN_VALUE. Actual: ' + (Number.MIN_VALUE / 1.1));
}

//CHECK#8
if (Number.MIN_VALUE / -1.1 !== -Number.MIN_VALUE) {
  host_assert_fail('#8: Number.MIN_VALUE / -1.1 === -Number.MIN_VALUE. Actual: ' + (Number.MIN_VALUE / -1.1));
}
}
