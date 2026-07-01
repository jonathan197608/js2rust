// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 7
// Source: test_statements_part*.js
// Run: node test_statements_frag_7.node.js

function testStatements_frag_7() {
    try {

        function getRectArea(width, height) {
          if (isNaN(width) || isNaN(height)) {
            throw new Error("Parameter is not a number!");
          }
        }

        try {
          getRectArea(3, "A");
        } catch (e) {
          console.error(e);
        }
        } catch (e) {
        console.error(`[testStatements_frag_7] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_7();
}

module.exports = { testStatements_frag_7 };
