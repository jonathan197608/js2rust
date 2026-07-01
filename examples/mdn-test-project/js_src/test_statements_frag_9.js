// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 9
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_9.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_9() {

        readFile("foo.txt", (err, data) => {
          if (err) {
            throw err;
          }
          console.log(data);
        });
    }
