// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 3
// Source: test_statements_part*.js
// Run: node test_statements_frag_3.node.js

function testStatements_frag_3() {
    try {

        let i = 0;

        while (i < 6) {
          if (i === 3) {
            break;
          }
          i += 1;
        }

        console.log(i);
        } catch (e) {
        console.error(`[testStatements_frag_3] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_3();
}

module.exports = { testStatements_frag_3 };
