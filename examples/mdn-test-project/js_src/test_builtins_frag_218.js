// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 218
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_218.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_218() {

        Object(0n) === 0n; // false
        Object(0n) === Object(0n); // false

        const o = Object(0n);
        o === o; // true
    }
