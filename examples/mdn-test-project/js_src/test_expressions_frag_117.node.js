// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 117
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_117.node.js

function testExpressions_frag_117() {
    try {

        Infinity % 2; // NaN
        Infinity % 0; // NaN
        Infinity % Infinity; // NaN
        2 % Infinity; // 2
        0 % Infinity; // 0
        } catch (e) {
        console.error(`[testExpressions_frag_117] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_117();
}

module.exports = { testExpressions_frag_117 };
