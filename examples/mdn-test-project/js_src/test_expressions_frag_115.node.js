// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 115
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_115.node.js

function testExpressions_frag_115() {
    try {

        -13 % 5; // -3
        -1 % 2; // -1
        -4 % 2; // -0

        -3n % 2n; // -1n
        } catch (e) {
        console.error(`[testExpressions_frag_115] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_115();
}

module.exports = { testExpressions_frag_115 };
