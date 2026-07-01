// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 23
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_23.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_23() {

        notHoisted(); // TypeError: notHoisted is not a function

        var notHoisted = function () {
          console.log("bar");
        };
    }
