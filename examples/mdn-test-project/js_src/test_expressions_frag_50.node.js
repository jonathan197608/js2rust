// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 50
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_50.node.js

function testExpressions_frag_50() {
    try {

        function A() {
          console.log("called A");
          return false;
        }
        function B() {
          console.log("called B");
          return true;
        }

        console.log(A() && B());
        // Logs "called A" to the console due to the call for function A,
        // && evaluates to false (function A returns false), then false is logged to the console;
        // the AND operator short-circuits here and ignores function B
        } catch (e) {
        console.error(`[testExpressions_frag_50] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_50();
}

module.exports = { testExpressions_frag_50 };
