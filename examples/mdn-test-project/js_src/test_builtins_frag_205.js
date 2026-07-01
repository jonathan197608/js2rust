// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 205
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_205.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_205() {

        Number("123"); // returns the number 123
        Number("123") === 123; // true

        Number("unicorn"); // NaN
        Number(undefined); // NaN
    }
