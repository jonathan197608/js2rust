// test262_language_expressions_strict_equals_S11_9_4_A5.js
// Source: test262/test/language/expressions/strict-equals/S11.9.4_A5.js

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

export function test262_language_expressions_strict_equals_S11_9_4_A5() {
// Copyright 2009 the Sputnik authors.  All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



//CHECK#1
if (!("" === "")) {
  host_assert_fail('#1: "" === ""');
}

//CHECK#2
if (!(" " === " ")) {
  host_assert_fail('#2: " " === " "');
}

//CHECK#3
if (!("string" === "string")) {
  host_assert_fail('#3: "string" === "string"');
}

//CHECK#4
if (" string" === "string ") {
  host_assert_fail('#4: " string" !== "string "');
}

//CHECK#5
if ("1.0" === "1") {
  host_assert_fail('#5: "1.0" !== "1"');
}
}
