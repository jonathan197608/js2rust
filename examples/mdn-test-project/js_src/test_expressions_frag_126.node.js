// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 126
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_126.node.js

function testExpressions_frag_126() {
    try {

        "foo" + "bar"; // "foobar"
        5 + "foo"; // "5foo"
        "foo" + false; // "foofalse"
        "2" + 2; // "22"
        } catch (e) {
        console.error(`[testExpressions_frag_126] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_126();
}

module.exports = { testExpressions_frag_126 };
