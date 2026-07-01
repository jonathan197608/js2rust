// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 70
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_70.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_70() {

        "abc".repeat(0); // ''
        "abc".repeat(1); // 'abc'
        "abc".repeat(2); // 'abcabc'
        "abc".repeat(3.5); // 'abcabcabc' (count will be converted to integer)
    }
