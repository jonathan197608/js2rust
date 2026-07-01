// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 100
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_100.node.js

function testExpressions_frag_100() {
    try {

        2 * 2; // 4
        -2 * 2; // -4

        Infinity * 0; // NaN
        Infinity * Infinity; // Infinity
        } catch (e) {
        console.error(`[testExpressions_frag_100] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_100();
}

module.exports = { testExpressions_frag_100 };
