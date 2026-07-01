// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 22
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_22.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_22() {

        hoisted(); // Logs "foo"

        function hoisted() {
          console.log("foo");
        }
    }
