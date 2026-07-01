// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 146
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_146.node.js

function testExpressions_frag_146() {
    try {

        5n <= 3; // false
        3 <= 3n; // true
        3 <= 5n; // true
        } catch (e) {
        console.error(`[testExpressions_frag_146] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_146();
}

module.exports = { testExpressions_frag_146 };
