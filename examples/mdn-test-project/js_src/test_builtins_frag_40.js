// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 40
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_40.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_40() {

        /(?:(a)|(ab))(?:(c)|(bc))/.exec("abc"); // ['abc', 'a', undefined, undefined, 'bc']
        // Not ['abc', undefined, 'ab', 'c', undefined]
    }
