// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 40
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_40.node.js

function testExpressions_frag_40() {
    try {

        // 9  (00000000000000000000000000001001)
        // 14 (00000000000000000000000000001110)

        14 & 9;
        // 8  (00000000000000000000000000001000)

        14n & 9n; // 8n
        } catch (e) {
        console.error(`[testExpressions_frag_40] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_40();
}

module.exports = { testExpressions_frag_40 };
