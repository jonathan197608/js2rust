// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 10
// Source: test_statements_part*.js
// Run: node test_statements_frag_10.node.js

function testStatements_frag_10() {
    try {

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
        } catch (e) {
        console.error(`[testStatements_frag_10] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_10();
}

module.exports = { testStatements_frag_10 };
