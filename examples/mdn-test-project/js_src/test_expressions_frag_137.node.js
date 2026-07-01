// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 137
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_137.node.js

function testExpressions_frag_137() {
    try {

        "5" > 3; // true
        "3" > 3; // false
        "3" > 5; // false

        "hello" > 5; // false
        5 > "hello"; // false

        "5" > 3n; // true
        "3" > 5n; // false
        } catch (e) {
        console.error(`[testExpressions_frag_137] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_137();
}

module.exports = { testExpressions_frag_137 };
