// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 35
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_35.node.js

function testExpressions_frag_35() {
    try {

        9 >>> 2; // 2
        -9 >>> 2; // 1073741821
        } catch (e) {
        console.error(`[testExpressions_frag_35] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_35();
}

module.exports = { testExpressions_frag_35 };
