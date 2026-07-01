// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 61
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_61.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_61() {

        true || false && false; // returns true, because && is executed first
        (true || false) && false; // returns false, because grouping has the highest precedence
    }
