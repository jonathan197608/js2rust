// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 109
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_109.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_109() {

        1n / 2n; // 0n
        5n / 3n; // 1n
        -1n / 3n; // 0n
        1n / -3n; // 0n

        2n / 0n; // RangeError: BigInt division by zero
    }
