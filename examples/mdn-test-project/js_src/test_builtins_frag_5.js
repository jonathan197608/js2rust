// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 5
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_5.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_5() {

        isNaN(1n); // TypeError: Conversion from 'BigInt' to 'number' is not allowed.
        Number.isNaN(1n); // false
    }
