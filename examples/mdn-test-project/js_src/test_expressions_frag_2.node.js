// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 2
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_2.node.js

function testExpressions_frag_2() {
    try {

        void 2 === "2"; // (void 2) === '2', returns false
        void (2 === "2"); // void (2 === '2'), returns undefined
        } catch (e) {
        console.error(`[testExpressions_frag_2] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_2();
}

module.exports = { testExpressions_frag_2 };
