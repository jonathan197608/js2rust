// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 79
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_79.node.js

function testBuiltins_frag_79() {
    try {

        for (const value of array) {
          if (value === 5) {
            continue;
          }
          // do something with value
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_79] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_79();
}

module.exports = { testBuiltins_frag_79 };
