// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 19
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_19.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_19() {

        console.log(
          `'foo' name ${
            "foo" in globalThis ? "is" : "is not"
          } global. typeof foo is ${typeof foo}`,
        );
        if (false) {
          function foo() {
            return 1;
          }
        }

        // In Chrome:
        // 'foo' name is global. typeof foo is undefined
        //
        // In Firefox:
        // 'foo' name is global. typeof foo is undefined
        //
        // In Safari:
        // 'foo' name is global. typeof foo is function
    }
