// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 101
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_101.node.js

function testExpressions_frag_101() {
    try {

        "foo" * 2; // NaN
        "2" * 2; // 4
        } catch (e) {
        console.error(`[testExpressions_frag_101] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_101();
}

module.exports = { testExpressions_frag_101 };
