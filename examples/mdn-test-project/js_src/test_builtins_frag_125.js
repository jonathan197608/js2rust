// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 125
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_125.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_125() {

        obj.foo.bar; // "baz"
        // or alternatively
        obj["foo"]["bar"]; // "baz"

        // computed properties require square brackets
        obj.foo["bar" + i]; // "baz2"
        // or as template literal
        obj.foo[`bar${i}`]; // "baz2"
    }
