// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 24
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_24.node.js

function testExpressions_frag_24() {
    try {

        const a = 5; // 00000000000000000000000000000101
        const b = 2; // 00000000000000000000000000000010

        console.log(a << b); // 00000000000000000000000000010100
        } catch (e) {
        console.error(`[testExpressions_frag_24] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_24();
}

module.exports = { testExpressions_frag_24 };
