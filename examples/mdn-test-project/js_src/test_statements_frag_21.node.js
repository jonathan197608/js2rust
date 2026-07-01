// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 21
// Source: test_statements_part*.js
// Run: node test_statements_frag_21.node.js

function testStatements_frag_21() {
    try {

        "use strict";

        {
          foo(); // Logs "foo"
          function foo() {
            console.log("foo");
          }
        }

        console.log(
          `'foo' name ${
            "foo" in globalThis ? "is" : "is not"
          } global. typeof foo is ${typeof foo}`,
        );
        // 'foo' name is not global. typeof foo is undefined
        } catch (e) {
        console.error(`[testStatements_frag_21] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_21();
}

module.exports = { testStatements_frag_21 };
