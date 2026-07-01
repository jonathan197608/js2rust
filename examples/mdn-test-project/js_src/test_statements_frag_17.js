// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 17
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_17.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_17() {

        const result = /(a+)(b+)(c+)/.exec("aaabcc");
        const [, a, b, c] = result;
        console.log(a, b, c); // "aaa" "b" "cc"
    }
