// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 124
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_124.node.js

function testExpressions_frag_124() {
    try {

        "1" + 2n; // "12"
        } catch (e) {
        console.error(`[testExpressions_frag_124] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_124();
}

module.exports = { testExpressions_frag_124 };
