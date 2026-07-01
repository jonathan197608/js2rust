// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 107
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_107.node.js

function testExpressions_frag_107() {
    try {

        1 / 2; // 0.5
        Math.floor(3 / 2); // 1
        1.0 / 2.0; // 0.5

        2 / 0; // Infinity
        2.0 / 0.0; // Infinity, because 0.0 === 0
        2.0 / -0.0; // -Infinity
        } catch (e) {
        console.error(`[testExpressions_frag_107] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_107();
}

module.exports = { testExpressions_frag_107 };
