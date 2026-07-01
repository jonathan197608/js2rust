// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 16
// Source: test_statements_part*.js
// Run: node test_statements_frag_16.node.js

function testStatements_frag_16() {
    try {

        MY_ARRAY.push("A"); // ["A"]
        } catch (e) {
        console.error(`[testStatements_frag_16] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_16();
}

module.exports = { testStatements_frag_16 };
