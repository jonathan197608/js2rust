// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 26
// Source: test_statements_part*.js
// Run: node test_statements_frag_26.node.js

function testStatements_frag_26() {
    try {

        const array = [1, 2, 3];

        // Assign all array values to 0
        for (let i = 0; i < array.length; array[i++] = 0 /* empty statement */);

        console.log(array);
        } catch (e) {
        console.error(`[testStatements_frag_26] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_26();
}

module.exports = { testStatements_frag_26 };
