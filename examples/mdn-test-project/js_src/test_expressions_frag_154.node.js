// Auto-generated MDN test fragment (Node.js reference runner)
// Category: expressions, Fragment: 154
// Source: test_expressions_part*.js
// Run: node test_expressions_frag_154.node.js

function testExpressions_frag_154() {
    try {

        true >= false; // true
        true >= true; // true
        false >= true; // false

        true >= 0; // true
        true >= 1; // true

        null >= 0; // true
        1 >= null; // true

        undefined >= 3; // false
        3 >= undefined; // false

        3 >= NaN; // false
        NaN >= 3; // false
        } catch (e) {
        console.error(`[testExpressions_frag_154] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testExpressions_frag_154();
}

module.exports = { testExpressions_frag_154 };
