// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 21
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_21.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_21() {

        "use strict";

        {
          foo(); // Logs "foo"
          function foo() {
            console.log("foo");
          }
        }

        console.log(
          `'foo' name ${
            "foo" in globalThis ? "is" : "is not"
          } global. typeof foo is ${typeof foo}`,
        );
        // 'foo' name is not global. typeof foo is undefined
    }
