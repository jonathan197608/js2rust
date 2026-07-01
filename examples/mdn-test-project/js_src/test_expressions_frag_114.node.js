// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 114
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_114.node.js

function testExpressions_frag_114() {
    try {

        13 % 5; // 3
        1 % -2; // 1
        1 % 2; // 1
        2 % 3; // 2
        5.5 % 2; // 1.5

        3n % 2n; // 1n
        } catch (e) {
        console.error(`[testExpressions_frag_114] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_114();
}

module.exports = { testExpressions_frag_114 };
