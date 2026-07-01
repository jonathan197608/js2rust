// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 0
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_0.node.js

function testExpressions_frag_0() {
    try {

        const output = void 1;
        console.log(output);

        void console.log("expression evaluated");

        void (function iife() {
          console.log("iife is executed");
        })();

        void function test() {
          console.log("test function executed");
        };
        try {
          test();
        } catch (e) {
          console.log("test function is not defined");
        }
        } catch (e) {
        console.error(`[testExpressions_frag_0] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_0();
}

module.exports = { testExpressions_frag_0 };
