// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 36
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_36.node.js

function testExpressions_frag_36() {
    try {

        9n >>> 2n; // TypeError: BigInts have no unsigned right shift, use >> instead
        } catch (e) {
        console.error(`[testExpressions_frag_36] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_36();
}

module.exports = { testExpressions_frag_36 };
