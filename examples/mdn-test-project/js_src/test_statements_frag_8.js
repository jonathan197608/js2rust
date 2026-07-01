// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 8
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_8.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_8() {

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
    }
