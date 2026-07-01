// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 123
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_123.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_123() {

        1n + 2; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        2 + 1n; // TypeError: Cannot mix BigInt and other types, use explicit conversions
    }
