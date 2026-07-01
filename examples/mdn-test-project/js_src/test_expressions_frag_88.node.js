// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 88
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_88.node.js

function testExpressions_frag_88() {
    try {

        console.log(3 ** 4);

        console.log(10 ** -2);

        console.log(2 ** (3 ** 2));

        console.log((2 ** 3) ** 2);
        } catch (e) {
        console.error(`[testExpressions_frag_88] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_88();
}

module.exports = { testExpressions_frag_88 };
