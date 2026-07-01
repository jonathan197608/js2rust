// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 102
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_102.node.js

function testExpressions_frag_102() {
    try {

        2n * 2n; // 4n
        -2n * 2n; // -4n
        } catch (e) {
        console.error(`[testExpressions_frag_102] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_102();
}

module.exports = { testExpressions_frag_102 };
