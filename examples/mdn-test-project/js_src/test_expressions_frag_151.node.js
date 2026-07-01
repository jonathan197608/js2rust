// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 151
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_151.node.js

function testExpressions_frag_151() {
    try {

        "5" >= 3; // true
        "3" >= 3; // true
        "3" >= 5; // false

        "hello" >= 5; // false
        5 >= "hello"; // false
        } catch (e) {
        console.error(`[testExpressions_frag_151] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_151();
}

module.exports = { testExpressions_frag_151 };
