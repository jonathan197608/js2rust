// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 74
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_74.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_74() {

        let foo = 3n;
        foo -= 2n; // 1n
        foo -= 1; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }
