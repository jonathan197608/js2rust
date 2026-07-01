// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 103
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_103.node.js

function testExpressions_frag_103() {
    try {

        2n * 2; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        2 * 2n; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        } catch (e) {
        console.error(`[testExpressions_frag_103] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_103();
}

module.exports = { testExpressions_frag_103 };
