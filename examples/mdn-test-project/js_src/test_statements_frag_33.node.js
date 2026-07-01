// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 33
// Source: test_statements_part*.js
// Run: node test_statements_frag_33.node.js

function testStatements_frag_33() {
    try {

        console.log(getPrimes(10)); // [2, 3, 5, 7]
        } catch (e) {
        console.error(`[testStatements_frag_33] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_33();
}

module.exports = { testStatements_frag_33 };
