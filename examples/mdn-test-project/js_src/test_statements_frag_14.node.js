// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 14
// Source: test_statements_part*.js
// Run: node test_statements_frag_14.node.js

function testStatements_frag_14() {
    try {

        MY_OBJECT.key = "otherValue";
        } catch (e) {
        console.error(`[testStatements_frag_14] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_14();
}

module.exports = { testStatements_frag_14 };
