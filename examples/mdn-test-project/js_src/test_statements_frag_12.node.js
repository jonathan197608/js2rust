// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 12
// Source: test_statements_part*.js
// Run: node test_statements_frag_12.node.js

function testStatements_frag_12() {
    try {

        // define MY_FAV as a constant and give it the value 7
        const MY_FAV = 7;

        console.log(`my favorite number is: ${MY_FAV}`);
        } catch (e) {
        console.error(`[testStatements_frag_12] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_12();
}

module.exports = { testStatements_frag_12 };
