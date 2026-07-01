// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 78
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_78.node.js

function testExpressions_frag_78() {
    try {

        let baz = 5;
        baz **= "foo"; // NaN
        } catch (e) {
        console.error(`[testExpressions_frag_78] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_78();
}

module.exports = { testExpressions_frag_78 };
