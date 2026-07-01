// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 150
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_150.node.js

function testExpressions_frag_150() {
    try {

        "a" >= "b"; // false
        "a" >= "a"; // true
        "a" >= "3"; // true
        } catch (e) {
        console.error(`[testExpressions_frag_150] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_150();
}

module.exports = { testExpressions_frag_150 };
