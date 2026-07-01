// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 11
// Source: test_statements_part*.js
// Run: node test_statements_frag_11.node.js

function testStatements_frag_11() {
    try {

        const number = 42;

        try {
          number = 99;
        } catch (err) {
          console.log(err);
          // (Note: the exact output may be browser-dependent)
        }

        console.log(number);
        } catch (e) {
        console.error(`[testStatements_frag_11] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_11();
}

module.exports = { testStatements_frag_11 };
