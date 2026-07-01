// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 45
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_45.node.js

function testExpressions_frag_45() {
    try {

        const a = 5; // 00000000000000000000000000000101
        const b = 3; // 00000000000000000000000000000011

        console.log(a ^ b); // 00000000000000000000000000000110
        } catch (e) {
        console.error(`[testExpressions_frag_45] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_45();
}

module.exports = { testExpressions_frag_45 };
