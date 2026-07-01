// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 140
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_140.node.js

function testExpressions_frag_140() {
    try {

        true > false; // true
        false > true; // false

        true > 0; // true
        true > 1; // false

        null > 0; // false
        1 > null; // true

        undefined > 3; // false
        3 > undefined; // false

        3 > NaN; // false
        NaN > 3; // false
        } catch (e) {
        console.error(`[testExpressions_frag_140] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_140();
}

module.exports = { testExpressions_frag_140 };
