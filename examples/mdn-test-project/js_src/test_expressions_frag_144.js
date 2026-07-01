// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 144
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_144.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_144() {

        "5" <= 3; // false
        "3" <= 3; // true
        "3" <= 5; // true

        "hello" <= 5; // false
        5 <= "hello"; // false
    }
