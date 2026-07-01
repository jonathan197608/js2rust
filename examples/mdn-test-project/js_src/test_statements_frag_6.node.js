// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 6
// Source: test_statements_part*.js
// Run: node test_statements_frag_6.node.js

function testStatements_frag_6() {
    try {

        outerBlock: {
          innerBlock: {
            console.log("1");
            break outerBlock; // breaks out of both innerBlock and outerBlock
            console.log(":-("); // skipped
          }
          console.log("2"); // skipped
        }
        } catch (e) {
        console.error(`[testStatements_frag_6] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_6();
}

module.exports = { testStatements_frag_6 };
