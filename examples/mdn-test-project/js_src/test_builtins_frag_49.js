// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 49
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_49.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_49() {

        /a*/.exec("aaa"); // ['aaa']; the entire input is consumed
        /a*?/.exec("aaa"); // ['']; it's possible to consume no characters and still match successfully
        /^a*?$/.exec("aaa"); // ['aaa']; it's not possible to consume fewer characters and still match successfully
    }
