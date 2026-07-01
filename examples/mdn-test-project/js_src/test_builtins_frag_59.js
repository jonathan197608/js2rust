// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 59
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_59.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_59() {

        String.fromCodePoint("_"); // RangeError
        String.fromCodePoint(Infinity); // RangeError
        String.fromCodePoint(-1); // RangeError
        String.fromCodePoint(3.14); // RangeError
        String.fromCodePoint(3e-2); // RangeError
        String.fromCodePoint(NaN); // RangeError
    }
