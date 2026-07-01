// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 109
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_109.node.js

function testExpressions_frag_109() {
    try {

        1n / 2n; // 0n
        5n / 3n; // 1n
        -1n / 3n; // 0n
        1n / -3n; // 0n

        2n / 0n; // RangeError: BigInt division by zero
        } catch (e) {
        console.error(`[testExpressions_frag_109] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_109();
}

module.exports = { testExpressions_frag_109 };
