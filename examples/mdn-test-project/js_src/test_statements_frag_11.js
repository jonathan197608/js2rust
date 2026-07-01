// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 11
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_11.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_11() {

        const number = 42;

        try {
          number = 99;
        } catch (err) {
          console.log(err);
          // (Note: the exact output may be browser-dependent)
        }

        console.log(number);
    }
