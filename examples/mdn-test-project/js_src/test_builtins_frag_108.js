// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 108
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_108.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_108() {

        isFinite(Infinity); // false
        isFinite(NaN); // false
        isFinite(-Infinity); // false

        isFinite(0); // true
        isFinite(2e64); // true
        isFinite(910); // true

        // Would've been false with the more robust Number.isFinite():
        isFinite(null); // true
        isFinite("0"); // true
    }
