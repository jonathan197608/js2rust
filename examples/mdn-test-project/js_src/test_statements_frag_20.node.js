// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 20
// Source: test_statements_part*.js
// Run: node test_statements_frag_20.node.js

function testStatements_frag_20() {
    try {

        console.log(
          `'foo' name ${
            "foo" in globalThis ? "is" : "is not"
          } global. typeof foo is ${typeof foo}`,
        );
        if (true) {
          function foo() {
            return 1;
          }
        }

        // In Chrome:
        // 'foo' name is global. typeof foo is undefined
        //
        // In Firefox:
        // 'foo' name is global. typeof foo is undefined
        //
        // In Safari:
        // 'foo' name is global. typeof foo is function
        } catch (e) {
        console.error(`[testStatements_frag_20] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_20();
}

module.exports = { testStatements_frag_20 };
