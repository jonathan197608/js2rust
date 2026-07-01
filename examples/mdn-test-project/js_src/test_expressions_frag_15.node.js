// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 15
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_15.node.js

function testExpressions_frag_15() {
    try {

        "3" === 3; // false
        true === 1; // false
        null === undefined; // false
        3 === new Number(3); // false
        } catch (e) {
        console.error(`[testExpressions_frag_15] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_15();
}

module.exports = { testExpressions_frag_15 };
