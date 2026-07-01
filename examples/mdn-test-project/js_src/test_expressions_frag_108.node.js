// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 108
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_108.node.js

function testExpressions_frag_108() {
    try {

        5 / "2"; // 2.5
        5 / "foo"; // NaN
        } catch (e) {
        console.error(`[testExpressions_frag_108] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_108();
}

module.exports = { testExpressions_frag_108 };
