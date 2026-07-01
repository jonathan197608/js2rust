// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 111
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_111.node.js

function testExpressions_frag_111() {
    try {

        2n / BigInt(2); // 1n
        Number(2n) / 2; // 1
        } catch (e) {
        console.error(`[testExpressions_frag_111] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_111();
}

module.exports = { testExpressions_frag_111 };
