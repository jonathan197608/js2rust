// runtime_test.js — Tier 3 runtime library integration tests

// === Math (Tier 1) test ===
function absValue() {
    return Math.abs(-42);
}

// === parseInt test (Tier 2) ===
function parseNumber() {
    return parseInt("256");
}

const test_absValue = absValue();
const test_parseNumber = parseNumber();
