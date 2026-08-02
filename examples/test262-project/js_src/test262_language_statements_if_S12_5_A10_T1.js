// test262_language_statements_if_S12_5_A10_T1.js
// Source: test262/test/language/statements/if/S12.5_A10_T1.js

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

export function test262_language_statements_if_S12_5_A10_T1() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//////////////////////////////////////////////////////////////////////////////
//CHECK#
if(function __func(){return 0;}){
    ;
}else {
    host_assert_fail('#1: Function expession inside the "if" expression is allowed');
}
//
//////////////////////////////////////////////////////////////////////////////
}
