// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 2
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_2.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_2() {
        // Auto-generated call for unused function
        calc(1);

        function magic() {
          return function calc(x) {
            return x * 42;
          };
        }

        const answer = magic();
        answer(1337); // 56154
    }
