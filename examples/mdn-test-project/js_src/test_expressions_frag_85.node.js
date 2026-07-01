// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 85
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_85.node.js

function testExpressions_frag_85() {
    try {

        !true; // !t returns false
        !false; // !f returns true
        !""; // !f returns true
        !"Cat"; // !t returns false
        } catch (e) {
        console.error(`[testExpressions_frag_85] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_85();
}

module.exports = { testExpressions_frag_85 };
