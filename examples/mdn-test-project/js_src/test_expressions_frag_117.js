// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 117
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_117.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_117() {

        Infinity % 2; // NaN
        Infinity % 0; // NaN
        Infinity % Infinity; // NaN
        2 % Infinity; // 2
        0 % Infinity; // 0
    }
