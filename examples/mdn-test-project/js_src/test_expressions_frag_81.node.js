// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 81
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_81.node.js

function testExpressions_frag_81() {
    try {

        typeof null; // "object" (not "null" for legacy reasons)
        typeof undefined; // "undefined"
        null === undefined; // false
        null == undefined; // true
        null === null; // true
        null == null; // true
        !null; // true
        Number.isNaN(1 + null); // false
        Number.isNaN(1 + undefined); // true
        } catch (e) {
        console.error(`[testExpressions_frag_81] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_81();
}

module.exports = { testExpressions_frag_81 };
