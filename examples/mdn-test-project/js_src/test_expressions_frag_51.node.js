// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 51
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_51.node.js

function testExpressions_frag_51() {
    try {

        true || false && false; // true
        true && (false || false); // false
        (2 === 3) || (4 < 0) && (1 === 1); // false
        } catch (e) {
        console.error(`[testExpressions_frag_51] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_51();
}

module.exports = { testExpressions_frag_51 };
