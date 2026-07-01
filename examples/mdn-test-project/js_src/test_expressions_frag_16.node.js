// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 16
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_16.node.js

function testExpressions_frag_16() {
    try {

        const object1 = {
          key: "value",
        };

        const object2 = {
          key: "value",
        };

        console.log(object1 === object2); // false
        console.log(object1 === object1); // true
        } catch (e) {
        console.error(`[testExpressions_frag_16] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_16();
}

module.exports = { testExpressions_frag_16 };
