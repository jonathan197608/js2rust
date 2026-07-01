// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 24
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_24.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_24() {

        function foo(a) {
          function a() {}
          console.log(typeof a);
        }

        foo(2); // Logs "function"
    }
