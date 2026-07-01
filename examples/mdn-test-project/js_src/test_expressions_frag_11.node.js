// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 11
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_11.node.js

function testExpressions_frag_11() {
    try {

        const object1 = {
          key: "value",
        };

        const object2 = {
          key: "value",
        };

        console.log(object1 != object2); // true
        console.log(object1 != object1); // false
        } catch (e) {
        console.error(`[testExpressions_frag_11] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_11();
}

module.exports = { testExpressions_frag_11 };
