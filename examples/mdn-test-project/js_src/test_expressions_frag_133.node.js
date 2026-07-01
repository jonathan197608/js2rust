// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 133
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_133.node.js

function testExpressions_frag_133() {
    try {

        2n - BigInt(1); // 1n
        Number(2n) - 1; // 1
        } catch (e) {
        console.error(`[testExpressions_frag_133] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_133();
}

module.exports = { testExpressions_frag_133 };
