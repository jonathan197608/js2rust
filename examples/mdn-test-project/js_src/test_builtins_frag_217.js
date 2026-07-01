// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 217
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_217.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_217() {

        const mixed = [4n, 6, -12n, 10, 4, 0, 0n];
        // [4n, 6, -12n, 10, 4, 0, 0n]

        mixed.sort(); // default sorting behavior
        // [ -12n, 0, 0n, 10, 4n, 4, 6 ]

        mixed.sort((a, b) => a - b);
        // won't work since subtraction will not work with mixed types
        // TypeError: can't convert BigInt value to Number value

        // sort with an appropriate numeric comparator
        mixed.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
        // [ -12n, 0, 0n, 4n, 4, 6, 10 ]
    }
