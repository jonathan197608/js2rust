// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 25
// Source: test_statements_part*.js
// Run: node test_statements_frag_25.node.js

function testStatements_frag_25() {
    try {

        function calcSales(unitsA, unitsB, unitsC) {
          return unitsA * 79 + unitsB * 129 + unitsC * 699;
        }
        } catch (e) {
        console.error(`[testStatements_frag_25] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_25();
}

module.exports = { testStatements_frag_25 };
