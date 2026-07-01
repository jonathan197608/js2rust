// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 14
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_14.node.js

function testExpressions_frag_14() {
    try {

        "hello" === "hello"; // true
        "hello" === "hola"; // false

        3 === 3; // true
        3 === 4; // false

        true === true; // true
        true === false; // false

        null === null; // true
        } catch (e) {
        console.error(`[testExpressions_frag_14] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_14();
}

module.exports = { testExpressions_frag_14 };
