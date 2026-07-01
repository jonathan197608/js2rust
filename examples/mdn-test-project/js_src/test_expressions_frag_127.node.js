// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 127
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_127.node.js

function testExpressions_frag_127() {
    try {

        console.log(5 - 3);

        console.log(3.5 - 5);

        console.log(5 - "hello");

        console.log(5 - true);
        } catch (e) {
        console.error(`[testExpressions_frag_127] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_127();
}

module.exports = { testExpressions_frag_127 };
