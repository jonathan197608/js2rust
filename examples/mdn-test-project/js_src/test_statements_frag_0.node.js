// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 0
// Run: node test_statements_frag_0.node.js

function testStatements_frag_0() {
    try {
        function getRectArea(width, height) {
          if (width > 0 && height > 0) {
            return width * height;
          }
          return 0;
        }

        console.log(getRectArea(3, 4));

        console.log(getRectArea(-3, 4));
    } catch (e) {
        console.error(`[testStatements_frag_0] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_0();
}

module.exports = { testStatements_frag_0 };
