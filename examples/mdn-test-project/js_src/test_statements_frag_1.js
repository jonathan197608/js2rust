// Auto-generated MDN test fragment (Zig transpile target)
// Category: statements, Fragment: 1
// Source: test_statements_part*.js
// Run with Node.js: node test_statements_frag_1.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testStatements_frag_1() {

        function counter() {
          // Infinite loop
          for (let count = 1; ; count++) {
            console.log(`${count}A`); // Until 5
            if (count === 5) {
              return;
            }
            console.log(`${count}B`); // Until 4
          }
          console.log(`${count}C`); // Never appears
        }

        counter();

        // Logs:
        // 1A
        // 1B
        // 2A
        // 2B
        // 3A
        // 3B
        // 4A
        // 4B
        // 5A
    }
