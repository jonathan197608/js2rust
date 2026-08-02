// test262_language_statements_if_S12_5_A1_T2.js
// Source: test262/test/language/statements/if/S12.5_A1_T2.js

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

export function test262_language_statements_if_S12_5_A1_T2() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



var c=0;
//////////////////////////////////////////////////////////////////////////////
//CHECK#1
if(!(1))
	host_assert_fail('#1.1: 1 in expression is evaluated to true');
else
  c++;
if (c!=1) host_assert_fail('#1.2: else branch don`t execute');
//
//////////////////////////////////////////////////////////////////////////////

//////////////////////////////////////////////////////////////////////////////
//CHECK#2
if(!(true))
	host_assert_fail('#2.1: true in expression is evaluated to true');
else
  c++;
if (c!=2) host_assert_fail('#2.2: else branch don`t execute');
//
//////////////////////////////////////////////////////////////////////////////

//////////////////////////////////////////////////////////////////////////////
//CHECK#3
if(!("1"))
	host_assert_fail('#3.1: "1" in expression is evaluated to true');
else
  c++;
if (c!=3) host_assert_fail('#3.2: else branch don`t execute');
//
//////////////////////////////////////////////////////////////////////////////

//////////////////////////////////////////////////////////////////////////////
//CHECK#4
if(!("A"))
	host_assert_fail('#4.1: "A" in expression is evaluated to true');
else
  c++;
if (c!=4) host_assert_fail('#4.2: else branch don`t execute');
//
//////////////////////////////////////////////////////////////////////////////
}
