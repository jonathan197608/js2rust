// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 82
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_82.node.js

function testExpressions_frag_82() {
    try {

        ~0; // -1
        ~-1; // 0
        ~1; // -2

        ~0n; // -1n
        ~4294967295n; // -4294967296n
        } catch (e) {
        console.error(`[testExpressions_frag_82] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_82();
}

module.exports = { testExpressions_frag_82 };
