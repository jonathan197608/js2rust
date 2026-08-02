// test262_language_statements_if_S12_5_A12_T1.js
// Source: test262/test/language/statements/if/S12.5_A12_T1.js

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

export function test262_language_statements_if_S12_5_A12_T1() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK# 1
if(true)
  if (false)
    host_assert_fail('#1.1: At embedded "if/else" constructions engine must select right branches');
  else
    ;
else 
  if (true)
    host_assert_fail('#1.2: At embedded "if/else" constructions engine must select right branches');
  else
    host_assert_fail('#1.3: At embedded "if/else" constructions engine must select right branches');

//CHECK# 2
if(true)
  if (true)
    ;
  else
    host_assert_fail('#2.1: At embedded "if/else" constructions engine must select right branches');
else 
  if (true)
    host_assert_fail('#2.2: At embedded "if/else" constructions engine must select right branches');
  else
    host_assert_fail('#2.3: At embedded "if/else" constructions engine must select right branches');

//CHECK# 3
if(false)
  if (true)
    host_assert_fail('#3.1: At embedded "if/else" constructions engine must select right branches');
  else
    host_assert_fail('#3.2: At embedded "if/else" constructions engine must select right branches');
else 
  if (true)
    ;
  else
    host_assert_fail('#3.3: At embedded "if/else" constructions engine must select right branches');

//CHECK# 4
if(false)
  if (true)
    host_assert_fail('#4.1: At embedded "if/else" constructions engine must select right branches');
  else
    host_assert_fail('#4.2: At embedded "if/else" constructions engine must select right branches');
else 
  if (false)
    host_assert_fail('#4.3: At embedded "if/else" constructions engine must select right branches');
  else
    ;
}
