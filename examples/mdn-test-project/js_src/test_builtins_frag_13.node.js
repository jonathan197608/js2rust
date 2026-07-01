// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 13
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_13.node.js

function testBuiltins_frag_13() {
    try {

        function random(min, max) {
          const num = Math.floor(Math.random() * (max - min + 1)) + min;
          return num;
        }

        random(1, 10);
        } catch (e) {
        console.error(`[testBuiltins_frag_13] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_13();
}

module.exports = { testBuiltins_frag_13 };
