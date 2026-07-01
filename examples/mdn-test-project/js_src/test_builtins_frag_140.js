// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 140
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_140.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_140() {

        Object.defineProperty({}, "key", 1);
        // TypeError: 1 is not a non-null object

        Object.defineProperty({}, "key", null);
        // TypeError: null is not a non-null object
    }
