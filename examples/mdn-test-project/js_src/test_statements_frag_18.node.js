// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 18
// Source: test_statements_part*.js
// Run: node test_statements_frag_18.node.js

function testStatements_frag_18() {
    try {

        function calcRectArea(width, height) {
          return width * height;
        }

        console.log(calcRectArea(5, 6));
        } catch (e) {
        console.error(`[testStatements_frag_18] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_18();
}

module.exports = { testStatements_frag_18 };
