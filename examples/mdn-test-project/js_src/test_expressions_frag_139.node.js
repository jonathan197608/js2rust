// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 139
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_139.node.js

function testExpressions_frag_139() {
    try {

        5n > 3; // true
        3 > 5n; // false
        } catch (e) {
        console.error(`[testExpressions_frag_139] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_139();
}

module.exports = { testExpressions_frag_139 };
