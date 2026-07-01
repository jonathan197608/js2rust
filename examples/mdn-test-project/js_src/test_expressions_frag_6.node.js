// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 6
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_6.node.js

function testExpressions_frag_6() {
    try {

        checkbox.onclick = () => void doSomething();
        } catch (e) {
        console.error(`[testExpressions_frag_6] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_6();
}

module.exports = { testExpressions_frag_6 };
