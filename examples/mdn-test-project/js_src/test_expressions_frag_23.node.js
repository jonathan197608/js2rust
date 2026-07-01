// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 23
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_23.node.js

function testExpressions_frag_23() {
    try {

        const object1 = {
          key: "value",
        };

        const object2 = {
          key: "value",
        };

        console.log(object1 !== object2); // true
        console.log(object1 !== object1); // false
        } catch (e) {
        console.error(`[testExpressions_frag_23] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_23();
}

module.exports = { testExpressions_frag_23 };
