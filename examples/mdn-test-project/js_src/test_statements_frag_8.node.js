// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 8
// Source: test_statements_part*.js
// Run: node test_statements_frag_8.node.js

function testStatements_frag_8() {
    try {

        function isNumeric(x) {
          return ["number", "bigint"].includes(typeof x);
        }

        function sum(...values) {
          if (!values.every(isNumeric)) {
            throw new TypeError("Can only add numbers");
          }
          return values.reduce((a, b) => a + b);
        }

        console.log(sum(1, 2, 3)); // 6
        try {
          sum("1", "2");
        } catch (e) {
          console.error(e); // TypeError: Can only add numbers
        }
        } catch (e) {
        console.error(`[testStatements_frag_8] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_8();
}

module.exports = { testStatements_frag_8 };
