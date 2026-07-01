// Auto-generated MDN test fragment (Zig transpile target)
// Category: expressions, Fragment: 140
// Source: test_expressions_part*.js
// Run with Node.js: node test_expressions_frag_140.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testExpressions_frag_140() {

        true > false; // true
        false > true; // false

        true > 0; // true
        true > 1; // false

        null > 0; // false
        1 > null; // true

        undefined > 3; // false
        3 > undefined; // false

        3 > NaN; // false
        NaN > 3; // false
    }
