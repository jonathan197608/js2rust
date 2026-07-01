// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 27
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_27.node.js

function testExpressions_frag_27() {
    try {

        9 << 3; // 72

        // 9 * (2 ** 3) = 9 * (8) = 72

        9n << 3n; // 72n
        } catch (e) {
        console.error(`[testExpressions_frag_27] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_27();
}

module.exports = { testExpressions_frag_27 };
