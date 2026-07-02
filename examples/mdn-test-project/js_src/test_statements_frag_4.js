// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 4
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_4.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_4() {
        // Auto-generated call for unused function
        testBreak(1);

        function testBreak(x) {
          let i = 0;

          while (i < 6) {
            if (i === 3) {
              break;
            }
            i += 1;
          }

          return i * x;
        }
    }
