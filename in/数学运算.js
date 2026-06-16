// Chinese filename: 数学运算.js → sanitized suffix: _u6570_u5b66_u8fd0_u7b97
// Internal helpers use this suffix for naming conflict resolution.

function helper(val) {
    return val * 2;
}

export function chineseAdd(a, b) {
    return a + b;
}

export function chineseSub(a, b) {
    return a - helper(b);
}

// test_* variables for Zig test generation (stripped from Zig output)
const test_chineseAdd = chineseAdd(3, 5);
const test_chineseSub = chineseSub(10, 3);
