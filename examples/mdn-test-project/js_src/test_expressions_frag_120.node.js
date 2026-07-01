// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 120
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_120.node.js

function testExpressions_frag_120() {
    try {

        const t = Temporal.Now.instant();
        "" + t; // Throws TypeError
        `${t}`; // '2022-07-31T04:48:56.113918308Z'
        "".concat(t); // '2022-07-31T04:48:56.113918308Z'
        } catch (e) {
        console.error(`[testExpressions_frag_120] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_120();
}

module.exports = { testExpressions_frag_120 };
