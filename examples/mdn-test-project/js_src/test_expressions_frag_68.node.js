// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 68
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_68.node.js

function testExpressions_frag_68() {
    try {

        let a = 3;

        console.log((a %= 2));

        console.log((a %= 0));

        console.log((a %= "hello"));
        } catch (e) {
        console.error(`[testExpressions_frag_68] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_68();
}

module.exports = { testExpressions_frag_68 };
