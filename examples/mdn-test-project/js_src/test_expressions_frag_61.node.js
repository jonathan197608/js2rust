// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 61
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_61.node.js

function testExpressions_frag_61() {
    try {

        true || false && false; // returns true, because && is executed first
        (true || false) && false; // returns false, because grouping has the highest precedence
        } catch (e) {
        console.error(`[testExpressions_frag_61] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_61();
}

module.exports = { testExpressions_frag_61 };
