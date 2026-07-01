// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 90
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_90.node.js

function testExpressions_frag_90() {
    try {

        2 ** 3; // 8
        3 ** 2; // 9
        3 ** 2.5; // 15.588457268119896
        10 ** -1; // 0.1
        2 ** 1024; // Infinity
        NaN ** 2; // NaN
        NaN ** 0; // 1
        1 ** Infinity; // NaN
        } catch (e) {
        console.error(`[testExpressions_frag_90] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_90();
}

module.exports = { testExpressions_frag_90 };
