// test262_language_expressions_division_S11_5_2_A4_T8.js
// Source: test262/test/language/expressions/division/S11.5.2_A4_T8.js

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

export function test262_language_expressions_division_S11_5_2_A4_T8() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (-0 / 1 !== -0) {
  host_assert_fail('#1.1: -0 / 1 === 0. Actual: ' + (-0 / 1));
} else {
  if (1 / (-0 / 1) !== Number.NEGATIVE_INFINITY) {
    host_assert_fail('#1.2: -0 / 1 === - 0. Actual: +0');
  }
}

//CHECK#2
if (-0 / -1 !== +0) {
  host_assert_fail('#2.1: -0 / -1 === 0. Actual: ' + (-0 / -1));
} else {
  if (1 / (-0 / -1) !== Number.POSITIVE_INFINITY) {
    host_assert_fail('#2.2: -0 / -1 === + 0. Actual: -0');
  }
}

//CHECK#3
if (+0 / 1 !== +0) {
  host_assert_fail('#3.1: +0 / 1 === 0. Actual: ' + (+0 / 1));
} else {
  if (1 / (+0 / 1) !== Number.POSITIVE_INFINITY) {
    host_assert_fail('#3.2: +0 / 1 === + 0. Actual: -0');
  }
}

//CHECK#4
if (+0 / -1 !== -0) {
  host_assert_fail('#4.1: +0 / -1 === 0. Actual: ' + (+0 / -1));
} else {
  if (1 / (+0 / -1) !== Number.NEGATIVE_INFINITY) {
    host_assert_fail('#4.2: +0 / -1 === - 0. Actual: +0');
  }
}

//CHECK#5
if (+0 / -Number.MAX_VALUE !== -0) {
  host_assert_fail('#5.1: 0 / -Number.MAX_VALUE === 0. Actual: ' + (0 / -Number.MAX_VALUE));
} else {
  if (1 / (+0 / -Number.MAX_VALUE) !== Number.NEGATIVE_INFINITY) {
    host_assert_fail('#5.2: +0 / -Number.MAX_VALUE === - 0. Actual: +0');
  }
}

//CHECK#6
if (-0 / Number.MIN_VALUE !== -0) {
  host_assert_fail('#6.1: -0 / Number.MIN_VALUE === 0. Actual: ' + (-0 / Number.MIN_VALUE));
} else {
  if (1 / (-0 / Number.MIN_VALUE) !== Number.NEGATIVE_INFINITY) {
    host_assert_fail('#6.2: -0 / Number.MIN_VALUE === - 0. Actual: +0');
  }
}
}
