// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 37
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_37.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_37() {

        /[\c0]/.test("\x10"); // true
        /[\c_]/.test("\x1f"); // true
        /[\c*]/.test("\\"); // true
        /\c/.test("\\c"); // true
        /\c0/.test("\\c0"); // true (the \c0 syntax is only supported in character classes)
    }
