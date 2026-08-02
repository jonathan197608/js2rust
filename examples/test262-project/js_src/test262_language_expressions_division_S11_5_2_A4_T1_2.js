// test262_language_expressions_division_S11_5_2_A4_T1_2.js
// Source: test262/test/language/expressions/division/S11.5.2_A4_T1.2.js

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

export function test262_language_expressions_division_S11_5_2_A4_T1_2() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (isNaN(Number.NaN / Number.NaN) !== true) {
  host_assert_fail('#1: NaN / NaN === Not-a-Number. Actual: ' + (NaN / NaN));
}  

//CHECK#2
if (isNaN(+0 / Number.NaN) !== true) {
  host_assert_fail('#2: +0 / NaN === Not-a-Number. Actual: ' + (+0 / NaN)); 
} 

//CHECK#3
if (isNaN(-0 / Number.NaN) !== true) {
  host_assert_fail('#3: -0 / NaN === Not-a-Number. Actual: ' + (-0 / NaN)); 
} 

//CHECK#4
if (isNaN(Number.POSITIVE_INFINITY / Number.NaN) !== true) {
  host_assert_fail('#4: Infinity / NaN === Not-a-Number. Actual: ' + (Infinity / NaN));
} 

//CHECK#5
if (isNaN(Number.NEGATIVE_INFINITY / Number.NaN) !== true) {
  host_assert_fail('#5:  -Infinity / NaN === Not-a-Number. Actual: ' + ( -Infinity / NaN)); 
} 

//CHECK#6
if (isNaN(Number.MAX_VALUE / Number.NaN) !== true) {
  host_assert_fail('#6: Number.MAX_VALUE / NaN === Not-a-Number. Actual: ' + (Number.MAX_VALUE / NaN));
} 

//CHECK#7
if (isNaN(Number.MIN_VALUE / Number.NaN) !== true) {
  host_assert_fail('#7: Number.MIN_VALUE / NaN === Not-a-Number. Actual: ' + (Number.MIN_VALUE / NaN)); 
}

//CHECK#8
if (isNaN(1 / Number.NaN) !== true) {
  host_assert_fail('#8: 1 / NaN === Not-a-Number. Actual: ' + (1 / NaN));  
}
}
