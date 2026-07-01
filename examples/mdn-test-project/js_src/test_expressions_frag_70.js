// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 70
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_70.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_70() {

        let bar = 5;

        bar %= 2; // 1
        bar %= "foo"; // NaN
        bar %= 0; // NaN

        let foo = 3n;
        foo %= 2n; // 1n
    }
