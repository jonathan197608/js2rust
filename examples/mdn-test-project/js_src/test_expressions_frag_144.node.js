// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 144
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_144.node.js

function testExpressions_frag_144() {
    try {

        "5" <= 3; // false
        "3" <= 3; // true
        "3" <= 5; // true

        "hello" <= 5; // false
        5 <= "hello"; // false
        } catch (e) {
        console.error(`[testExpressions_frag_144] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_144();
}

module.exports = { testExpressions_frag_144 };
