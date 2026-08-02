// test262_language_expressions_multiplication_S11_5_1_A4_T2.js
// Source: test262/test/language/expressions/multiplication/S11.5.1_A4_T2.js

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

export function test262_language_expressions_multiplication_S11_5_1_A4_T2() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (1 * 1 !== 1) {
  host_assert_fail('#1: 1 * 1 === 1. Actual: ' + (1 * 1));
}

//CHECK#2
if (1 * -1 !== -1) {
  host_assert_fail('#2: 1 * -1 === -1. Actual: ' + (1 * -1));
}

//CHECK#3
if (-1 * 1 !== -1) {
  host_assert_fail('#3: -1 * 1 === -1. Actual: ' + (-1 * 1));
}

//CHECK#4
if (-1 * -1 !== 1) {
  host_assert_fail('#4: -1 * -1 === 1. Actual: ' + (-1 * -1));
}

//CHECK#5
if (0 * 0 !== 0) {
  host_assert_fail('#5.1: 0 * 0 === 0. Actual: ' + (0 * 0));
} else {
  if (1 / (0 * 0) !== Number.POSITIVE_INFINITY) {
    host_assert_fail('#5.2: 0 * 0 === + 0. Actual: -0');
  }
}

//CHECK#6
if (0 * -0 !== -0) {
  host_assert_fail('#6.1: 0 * -0 === 0. Actual: ' + (0 * -0));
} else {
  if (1 / (0 * -0) !== Number.NEGATIVE_INFINITY) {
    host_assert_fail('#6.2: 0 * -0 === - 0. Actual: +0');
  }
}

//CHECK#7
if (-0 * 0 !== -0) {
  host_assert_fail('#7.1: -0 * 0 === 0. Actual: ' + (-0 * 0));
} else {
  if (1 / (-0 * 0) !== Number.NEGATIVE_INFINITY) {
    host_assert_fail('#7.2: -0 * 0 === - 0. Actual: +0');
  }
}

//CHECK#8
if (-0 * -0 !== 0) {
  host_assert_fail('#8.1: -0 * -0 === 0. Actual: ' + (-0 * -0));
} else {
  if (1 / (-0 * -0) !== Number.POSITIVE_INFINITY) {
    host_assert_fail('#8.2: 0 * -0 === - 0. Actual: +0');
  }
}
}
