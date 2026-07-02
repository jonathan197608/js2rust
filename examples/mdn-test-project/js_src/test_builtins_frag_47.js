// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 47
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_47.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_47() {

                const a = 1;
const re = /a{1, 3}/;
        re.test("aa"); // false
        re.test("a{1, 3}"); // true
    }
