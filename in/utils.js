// utils.js — utility functions module
// Exports: voidFunc
// Internal: helper (DELIBERATELY same name as strings.js helper — tests suffix naming)

export function voidFunc() {
}

// Internal helper — CONFLICT: same name in strings.js
// Preprocessing resolves via suffix: helper_utils, helper_strings
function helper(x) {
    return x;
}
