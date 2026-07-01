// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 148
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_148.node.js

function testExpressions_frag_148() {
    try {

        console.log(5 >= 3);

        console.log(3 >= 3);

        // Compare bigint to number
        console.log(3n >= 5);

        console.log("ab" >= "aa");
        } catch (e) {
        console.error(`[testExpressions_frag_148] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_148();
}

module.exports = { testExpressions_frag_148 };
