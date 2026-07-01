// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 99
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_99.node.js

function testBuiltins_frag_99() {
    try {

        start: {
          console.log("Hello, world!");
          if (Math.random() > 0.5) {
            break start;
          }
          console.log("Maybe I'm logged");
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_99] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_99();
}

module.exports = { testBuiltins_frag_99 };
