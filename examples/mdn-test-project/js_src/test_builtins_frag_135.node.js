// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 135
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_135.node.js

function testBuiltins_frag_135() {
    try {

        function f(arg) {
          arg = "foo";
        }

        function g(arg) {
          let bar = "foo";
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_135] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_135();
}

module.exports = { testBuiltins_frag_135 };
