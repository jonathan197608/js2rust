// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 28
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_28.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_28() {

        const a = 5; //  00000000000000000000000000000101
        const b = 2; //  00000000000000000000000000000010
        const c = -5; //  11111111111111111111111111111011

        console.log(a >> b); //  00000000000000000000000000000001

        console.log(c >> b); //  11111111111111111111111111111110
    }
