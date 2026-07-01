// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 105
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_105.node.js

function testExpressions_frag_105() {
    try {

        console.log(12 / 2);

        console.log(3 / 2);

        console.log(6 / "3");

        console.log(2 / 0);
        } catch (e) {
        console.error(`[testExpressions_frag_105] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_105();
}

module.exports = { testExpressions_frag_105 };
