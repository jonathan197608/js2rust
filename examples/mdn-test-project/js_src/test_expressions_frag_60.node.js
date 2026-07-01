// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 60
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_60.node.js

function testExpressions_frag_60() {
    try {

        function A() {
          console.log("called A");
          return false;
        }
        function B() {
          console.log("called B");
          return true;
        }

        console.log(B() || A());
        // Logs "called B" due to the function call,
        // then logs true (which is the resulting value of the operator)
        } catch (e) {
        console.error(`[testExpressions_frag_60] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_60();
}

module.exports = { testExpressions_frag_60 };
