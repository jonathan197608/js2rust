// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 28
// Source: test_statements_part*.js
// Run: node test_statements_frag_28.node.js

function testStatements_frag_28() {
    try {

        const arr = [1, 2, 3];

        // Assign all array values to 0
        for (let i = 0; i < arr.length; arr[i++] = 0) /* empty statement */ ;

        console.log(arr);
        // [0, 0, 0]
        } catch (e) {
        console.error(`[testStatements_frag_28] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_28();
}

module.exports = { testStatements_frag_28 };
