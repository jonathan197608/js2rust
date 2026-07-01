// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 123
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_123.node.js

function testExpressions_frag_123() {
    try {

        1n + 2; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        2 + 1n; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        } catch (e) {
        console.error(`[testExpressions_frag_123] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_123();
}

module.exports = { testExpressions_frag_123 };
