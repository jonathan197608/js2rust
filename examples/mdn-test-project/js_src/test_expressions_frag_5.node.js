// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 5
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_5.node.js

function testExpressions_frag_5() {
    try {

        checkbox.onclick = () => doSomething();
        } catch (e) {
        console.error(`[testExpressions_frag_5] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_5();
}

module.exports = { testExpressions_frag_5 };
