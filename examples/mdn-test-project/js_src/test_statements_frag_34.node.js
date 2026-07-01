// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 34
// Source: test_statements_part*.js
// Run: node test_statements_frag_34.node.js

function testStatements_frag_34() {
    try {

        // my-module.js
        let myValue = 1;
        setTimeout(() => {
          myValue = 2;
        }, 500);
        } catch (e) {
        console.error(`[testStatements_frag_34] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_34();
}

module.exports = { testStatements_frag_34 };
