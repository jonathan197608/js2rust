// test262_language_expressions_division_no_magic_asi.js
// Source: test262/test/language/expressions/division/no-magic-asi.js

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

export function test262_language_expressions_division_no_magic_asi() {
// Copyright (C) 2019 Leo Balter. All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.



var instance = 60;
var of = 6;
var g = 2;

var notRegExp = instance/of/g;

assert_same_value(notRegExp, 5, "");
}
