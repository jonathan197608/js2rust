// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 23
// Source: test_statements_part*.js
// Run: node test_statements_frag_23.node.js

function testStatements_frag_23() {
    try {

        notHoisted(); // TypeError: notHoisted is not a function

        var notHoisted = function () {
          console.log("bar");
        };
        } catch (e) {
        console.error(`[testStatements_frag_23] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_23();
}

module.exports = { testStatements_frag_23 };
