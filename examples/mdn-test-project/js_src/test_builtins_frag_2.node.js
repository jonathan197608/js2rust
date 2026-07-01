// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 2
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_2.node.js

function testBuiltins_frag_2() {
    try {

        function sanitize(x) {
          if (isNaN(x)) {
            return NaN;
          }
          return x;
        }

        console.log(sanitize("1"));

        console.log(sanitize("NotANumber"));
        } catch (e) {
        console.error(`[testBuiltins_frag_2] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_2();
}

module.exports = { testBuiltins_frag_2 };
