// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 39
// Source: test_statements_part*.js
// Run: node test_statements_frag_39.node.js

function testStatements_frag_39() {
    try {

        const objectHavingAnEspeciallyLengthyName = { foo: true, bar: false };

        if (((o) => o.foo && !o.bar)(objectHavingAnEspeciallyLengthyName)) {
          // This branch runs.
        }
        } catch (e) {
        console.error(`[testStatements_frag_39] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_39();
}

module.exports = { testStatements_frag_39 };
