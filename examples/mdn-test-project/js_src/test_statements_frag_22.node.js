// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 22
// Source: test_statements_part*.js
// Run: node test_statements_frag_22.node.js

function testStatements_frag_22() {
    try {

        hoisted(); // Logs "foo"

        function hoisted() {
          console.log("foo");
        }
        } catch (e) {
        console.error(`[testStatements_frag_22] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_22();
}

module.exports = { testStatements_frag_22 };
