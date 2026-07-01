// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 21
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_21.node.js

function testExpressions_frag_21() {
    try {

        "hello" !== "hello"; // false
        "hello" !== "hola"; // true

        3 !== 3; // false
        3 !== 4; // true

        true !== true; // false
        true !== false; // true

        null !== null; // false
        } catch (e) {
        console.error(`[testExpressions_frag_21] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_21();
}

module.exports = { testExpressions_frag_21 };
