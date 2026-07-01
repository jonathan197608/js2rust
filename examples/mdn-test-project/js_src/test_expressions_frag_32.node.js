// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 32
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_32.node.js

function testExpressions_frag_32() {
    try {

        const a = 5; //  00000000000000000000000000000101
        const b = 2; //  00000000000000000000000000000010
        const c = -5; //  11111111111111111111111111111011

        console.log(a >>> b); //  00000000000000000000000000000001

        console.log(c >>> b); //  00111111111111111111111111111110
        } catch (e) {
        console.error(`[testExpressions_frag_32] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_32();
}

module.exports = { testExpressions_frag_32 };
