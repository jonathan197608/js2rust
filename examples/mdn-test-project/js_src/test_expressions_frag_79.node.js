// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 79
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_79.node.js

function testExpressions_frag_79() {
    try {

        let foo = 3n;
        foo **= 2n; // 9n
        foo **= 1; // TypeError: Cannot mix BigInt and other types, use explicit conversions
        } catch (e) {
        console.error(`[testExpressions_frag_79] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_79();
}

module.exports = { testExpressions_frag_79 };
