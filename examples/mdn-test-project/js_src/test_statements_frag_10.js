// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 10
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_10.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_10() {

        (async () => {{
            function readFilePromise(path) {
              return new Promise((resolve, reject) => {
                readFile(path, (err, data) => {
                  if (err) {
                    reject(err);
                  }
                  resolve(data);
                });
              });
            }

            try {
              const data = await readFilePromise("foo.txt");
              console.log(data);
            } catch (err) {
              console.error(err);
            }
        }})();
    }
