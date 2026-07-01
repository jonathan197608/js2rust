// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 95
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_95.node.js

function testExpressions_frag_95() {
    try {

        2 ** 3 ** 2; // 512
        2 ** (3 ** 2); // 512
        (2 ** 3) ** 2; // 64
        } catch (e) {
        console.error(`[testExpressions_frag_95] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_95();
}

module.exports = { testExpressions_frag_95 };
