// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 130
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_130.node.js

function testExpressions_frag_130() {
    try {

        "foo" - 3; // NaN; "foo" is converted to the number NaN
        5 - "3"; // 2; "3" is converted to the number 3
        } catch (e) {
        console.error(`[testExpressions_frag_130] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_130();
}

module.exports = { testExpressions_frag_130 };
