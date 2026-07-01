// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 137
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_137.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_137() {

        "5" > 3; // true
        "3" > 3; // false
        "3" > 5; // false

        "hello" > 5; // false
        5 > "hello"; // false

        "5" > 3n; // true
        "3" > 5n; // false
    }
