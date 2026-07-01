// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 152
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_152.node.js

function testExpressions_frag_152() {
    try {

        5 >= 3; // true
        3 >= 3; // true
        3 >= 5; // false
        } catch (e) {
        console.error(`[testExpressions_frag_152] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_152();
}

module.exports = { testExpressions_frag_152 };
