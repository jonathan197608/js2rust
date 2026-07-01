// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 160
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_160.node.js

function testExpressions_frag_160() {
    try {

        1 != 2; // true
        "hello" != "hola"; // true

        1 != 1; // false
        "hello" != "hello"; // false
        } catch (e) {
        console.error(`[testExpressions_frag_160] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_160();
}

module.exports = { testExpressions_frag_160 };
