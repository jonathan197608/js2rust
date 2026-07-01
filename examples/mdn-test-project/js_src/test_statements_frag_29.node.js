// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 29
// Source: test_statements_part*.js
// Run: node test_statements_frag_29.node.js

function testStatements_frag_29() {
    try {

        if (condition);      // Caution, this "if" does nothing!
          killTheUniverse(); // So this always gets executed!!!
        } catch (e) {
        console.error(`[testStatements_frag_29] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_29();
}

module.exports = { testStatements_frag_29 };
