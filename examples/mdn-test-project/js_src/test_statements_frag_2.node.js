// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 2
// Source: test_statements_part*.js
// Run: node test_statements_frag_2.node.js

function testStatements_frag_2() {
    try {

        function magic() {
          return function calc(x) {
            return x * 42;
          };
        }

        const answer = magic();
        answer(1337); // 56154
        } catch (e) {
        console.error(`[testStatements_frag_2] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_2();
}

module.exports = { testStatements_frag_2 };
