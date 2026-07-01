// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 36
// Source: test_statements_part*.js
// Run: node test_statements_frag_36.node.js

function testStatements_frag_36() {
    try {

        foo; // unqualified identifier
        foo.bar; // bar is a qualified identifier
        } catch (e) {
        console.error(`[testStatements_frag_36] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_36();
}

module.exports = { testStatements_frag_36 };
