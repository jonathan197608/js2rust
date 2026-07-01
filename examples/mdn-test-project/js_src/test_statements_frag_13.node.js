// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 13
// Source: test_statements_part*.js
// Run: node test_statements_frag_13.node.js

function testStatements_frag_13() {
    try {

        const MY_OBJECT = { key: "value" };
        MY_OBJECT = { OTHER_KEY: "value" };
        } catch (e) {
        console.error(`[testStatements_frag_13] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_13();
}

module.exports = { testStatements_frag_13 };
