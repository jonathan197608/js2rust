// test262_language_expressions_subtraction_S11_6_2_A4_T6.js
// Source: test262/test/language/expressions/subtraction/S11.6.2_A4_T6.js

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

export function test262_language_expressions_subtraction_S11_6_2_A4_T6() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (1 - -0 !== 1 ) {  
  host_assert_fail('#1: 1 - -0 === 1. Actual: ' + (1 - -0));
}

//CHECK#2
if (1 - 0 !== 1 ) {  
  host_assert_fail('#2: 1 - 0 === 1. Actual: ' + (1 - 0));
} 

//CHECK#3
if (-0 - 1 !== -1 ) {  
  host_assert_fail('#3: -0 - 1 === -1. Actual: ' + (-0 - 1));
}

//CHECK#4
if (0 - 1 !== -1 ) {  
  host_assert_fail('#4: 0 - 1 === -1. Actual: ' + (0 - 1));
} 

//CHECK#5
if (Number.MAX_VALUE - -0 !== Number.MAX_VALUE ) {  
  host_assert_fail('#5: Number.MAX_VALUE - -0 === Number.MAX_VALUE. Actual: ' + (Number.MAX_VALUE - -0));
}

//CHECK#6
if (Number.MAX_VALUE - 0 !== Number.MAX_VALUE ) {  
  host_assert_fail('#6: Number.MAX_VALUE - 0 === Number.MAX_VALUE. Actual: ' + (Number.MAX_VALUE - 0));
} 

//CHECK#7
if (-0 - Number.MIN_VALUE !== -Number.MIN_VALUE ) {  
  host_assert_fail('#7: -0 - Number.MIN_VALUE === -Number.MIN_VALUE. Actual: ' + (-0 - Number.MIN_VALUE));
}

//CHECK#8
if (0 - Number.MIN_VALUE !== -Number.MIN_VALUE ) {  
  host_assert_fail('#8: 0 - Number.MIN_VALUE === -Number.MIN_VALUE. Actual: ' + (0 - Number.MIN_VALUE));
}
}
