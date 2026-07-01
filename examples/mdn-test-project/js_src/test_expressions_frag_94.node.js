// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 94
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_94.node.js

function testExpressions_frag_94() {
    try {

        2n ** BigInt(2); // 4n
        Number(2n) ** 2; // 4
        } catch (e) {
        console.error(`[testExpressions_frag_94] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_94();
}

module.exports = { testExpressions_frag_94 };
