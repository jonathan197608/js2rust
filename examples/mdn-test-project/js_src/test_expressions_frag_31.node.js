// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 31
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_31.node.js

function testExpressions_frag_31() {
    try {

        9 >> 2; // 2
        -9 >> 2; // -3

        9n >> 2n; // 2n
        } catch (e) {
        console.error(`[testExpressions_frag_31] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_31();
}

module.exports = { testExpressions_frag_31 };
