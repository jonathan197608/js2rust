// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 7
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_7.node.js

function testExpressions_frag_7() {
    try {

        const a = 5; // 00000000000000000000000000000101
        const b = -3; // 11111111111111111111111111111101

        console.log(~a); // 11111111111111111111111111111010

        console.log(~b); // 00000000000000000000000000000010
        } catch (e) {
        console.error(`[testExpressions_frag_7] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_7();
}

module.exports = { testExpressions_frag_7 };
