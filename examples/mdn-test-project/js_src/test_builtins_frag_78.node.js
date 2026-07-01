// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 78
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_78.node.js

function testBuiltins_frag_78() {
    try {

        array.forEach((value) => {
          if (value === 5) {
            return;
          }
          // do something with value
        });
        } catch (e) {
        console.error(`[testBuiltins_frag_78] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_78();
}

module.exports = { testBuiltins_frag_78 };
