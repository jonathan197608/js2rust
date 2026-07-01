// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 24
// Source: test_statements_part*.js
// Run: node test_statements_frag_24.node.js

function testStatements_frag_24() {
    try {

        function foo(a) {
          function a() {}
          console.log(typeof a);
        }

        foo(2); // Logs "function"
        } catch (e) {
        console.error(`[testStatements_frag_24] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_24();
}

module.exports = { testStatements_frag_24 };
