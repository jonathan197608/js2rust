// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 9
// Source: test_statements_part*.js
// Run: node test_statements_frag_9.node.js

function testStatements_frag_9() {
    try {

        readFile("foo.txt", (err, data) => {
          if (err) {
            throw err;
          }
          console.log(data);
        });
        } catch (e) {
        console.error(`[testStatements_frag_9] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_9();
}

module.exports = { testStatements_frag_9 };
