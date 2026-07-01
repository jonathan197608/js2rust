// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 37
// Source: test_statements_part*.js
// Run: node test_statements_frag_37.node.js

function testStatements_frag_37() {
    try {

        const foo = { bar: 1 };
        console.log(foo.bar);
        // foo is found in the scope chain as a variable;
        // bar is found in foo as a property
        } catch (e) {
        console.error(`[testStatements_frag_37] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_37();
}

module.exports = { testStatements_frag_37 };
