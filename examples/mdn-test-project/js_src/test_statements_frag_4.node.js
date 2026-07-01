// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 4
// Source: test_statements_part*.js
// Run: node test_statements_frag_4.node.js

function testStatements_frag_4() {
    try {

        function testBreak(x) {
          let i = 0;

          while (i < 6) {
            if (i === 3) {
              break;
            }
            i += 1;
          }

          return i * x;
        }
        } catch (e) {
        console.error(`[testStatements_frag_4] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_4();
}

module.exports = { testStatements_frag_4 };
