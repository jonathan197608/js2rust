// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 48
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_48.node.js

function testExpressions_frag_48() {
    try {

        // 9  (00000000000000000000000000001001)
        // 14 (00000000000000000000000000001110)

        14 ^ 9;
        // 7  (00000000000000000000000000000111)

        14n ^ 9n; // 7n
        } catch (e) {
        console.error(`[testExpressions_frag_48] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_48();
}

module.exports = { testExpressions_frag_48 };
