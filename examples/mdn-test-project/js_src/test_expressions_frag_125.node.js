// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 125
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_125.node.js

function testExpressions_frag_125() {
    try {

        1n + BigInt(2); // 3n
        Number(1n) + 2; // 3
        } catch (e) {
        console.error(`[testExpressions_frag_125] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_125();
}

module.exports = { testExpressions_frag_125 };
