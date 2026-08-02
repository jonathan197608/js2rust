// test262_language_expressions_addition_S11_6_1_A3_1_T1_3.js
// Source: test262/test/language/expressions/addition/S11.6.1_A3.1_T1.3.js

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

export function test262_language_expressions_addition_S11_6_1_A3_1_T1_3() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (isNaN(null + undefined) !== true) {
  host_assert_fail('#1: null + undefined === Not-a-Number. Actual: ' + (null + undefined));
}

//CHECK#2
if (isNaN(undefined + null) !== true) {
  host_assert_fail('#2: undefined + null === Not-a-Number. Actual: ' + (undefined + null));
}

//CHECK#3
if (isNaN(undefined + undefined) !== true) {
  host_assert_fail('#3: undefined + undefined === Not-a-Number. Actual: ' + (undefined + undefined));
}

//CHECK#4
if (null + null !== 0) {
  host_assert_fail('#4: null + null === 0. Actual: ' + (null + null));
}
}
