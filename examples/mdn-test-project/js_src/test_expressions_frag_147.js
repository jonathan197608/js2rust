// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 147
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_147.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_147() {

        true <= false; // false
        true <= true; // true
        false <= true; // true

        true <= 0; // false
        true <= 1; // true

        null <= 0; // true
        1 <= null; // false

        undefined <= 3; // false
        3 <= undefined; // false

        3 <= NaN; // false
        NaN <= 3; // false
    }
