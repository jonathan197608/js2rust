// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 7
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_7.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_7() {

        function getRectArea(width, height) {
          if (isNaN(width) || isNaN(height)) {
            throw new Error("Parameter is not a number!");
          }
        }

        try {
          getRectArea(3, "A");
        } catch (e) {
          console.error(e);
        }
    }
