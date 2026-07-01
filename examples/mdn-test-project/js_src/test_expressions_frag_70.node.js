// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 70
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_70.node.js

function testExpressions_frag_70() {
    try {

        let bar = 5;

        bar %= 2; // 1
        bar %= "foo"; // NaN
        bar %= 0; // NaN

        let foo = 3n;
        foo %= 2n; // 1n
        } catch (e) {
        console.error(`[testExpressions_frag_70] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_70();
}

module.exports = { testExpressions_frag_70 };
