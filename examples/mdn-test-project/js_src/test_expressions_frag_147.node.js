// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 147
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_147.node.js

function testExpressions_frag_147() {
    try {

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
        } catch (e) {
        console.error(`[testExpressions_frag_147] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_147();
}

module.exports = { testExpressions_frag_147 };
