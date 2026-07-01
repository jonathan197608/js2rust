// Auto-generated MDN test fragment (Node.js reference runner)
// Category: statements, Fragment: 1
// Source: test_statements_part*.js
// Run: node test_statements_frag_1.node.js

function testStatements_frag_1() {
    try {

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
        } catch (e) {
        console.error(`[testStatements_frag_1] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testStatements_frag_1();
}

module.exports = { testStatements_frag_1 };
