// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 9
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_9.node.js

function testBuiltins_frag_9() {
    try {

        function div(x) {
          if (isFinite(1000 / x)) {
            return "Number is NOT Infinity.";
          }
          return "Number is Infinity!";
        }

        console.log(div(0));

        console.log(div(1));
        } catch (e) {
        console.error(`[testBuiltins_frag_9] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_9();
}

module.exports = { testBuiltins_frag_9 };
