// strings.js — string operations module
// Exports: greet
// Internal: helper (CONFLICT with utils.js helper — tests suffix naming)

export function greet(name) {
    return "hello " + name;
}

// Internal helper — CONFLICT: same name in utils.js
// Preprocessing resolves via suffix: helper_utils, helper_strings
function helper(x) {
    return x;
}
