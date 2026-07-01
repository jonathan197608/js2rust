// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 22
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_22.node.js

function testExpressions_frag_22() {
    try {

        "3" !== 3; // true
        true !== 1; // true
        null !== undefined; // true
        } catch (e) {
        console.error(`[testExpressions_frag_22] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_22();
}

module.exports = { testExpressions_frag_22 };
