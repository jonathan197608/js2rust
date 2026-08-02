// test262_language_expressions_addition_S11_6_1_A4_T9.js
// Source: test262/test/language/expressions/addition/S11.6.1_A4_T9.js

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

export function test262_language_expressions_addition_S11_6_1_A4_T9() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (-Number.MAX_VALUE + Number.MAX_VALUE + Number.MAX_VALUE !== (-Number.MAX_VALUE + Number.MAX_VALUE) + Number.MAX_VALUE) {
  host_assert_fail('#1: -Number.MAX_VALUE + Number.MAX_VALUE + Number.MAX_VALUE === (-Number.MAX_VALUE + Number.MAX_VALUE) + Number.MAX_VALUE. Actual: ' + (-Number.MAX_VALUE + Number.MAX_VALUE + Number.MAX_VALUE));
} 

//CHECK#2
if ((-Number.MAX_VALUE + Number.MAX_VALUE) + Number.MAX_VALUE === -Number.MAX_VALUE + (Number.MAX_VALUE + Number.MAX_VALUE)) {
  host_assert_fail('#2: (-Number.MAX_VALUE + Number.MAX_VALUE) + Number.MAX_VALUE === -Number.MAX_VALUE + (Number.MAX_VALUE + Number.MAX_VALUE). Actual: ' + ((-Number.MAX_VALUE + Number.MAX_VALUE) + Number.MAX_VALUE));
}

//CHECK#3
if ("1" + 1 + 1 !== ("1" + 1) + 1) {
  host_assert_fail('#3: "1" + 1 + 1 === ("1" + 1) + 1. Actual: ' + ("1" + 1 + 1));
} 

//CHECK#4
if (("1" + 1) + 1 === "1" + (1 + 1)) {
  host_assert_fail('#4: ("1" + 1) + 1 !== "1" + (1 + 1)');
}
}
