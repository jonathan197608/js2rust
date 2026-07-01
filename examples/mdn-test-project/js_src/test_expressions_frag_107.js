// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 107
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_107.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_107() {

        1 / 2; // 0.5
        Math.floor(3 / 2); // 1
        1.0 / 2.0; // 0.5

        2 / 0; // Infinity
        2.0 / 0.0; // Infinity, because 0.0 === 0
        2.0 / -0.0; // -Infinity
    }
