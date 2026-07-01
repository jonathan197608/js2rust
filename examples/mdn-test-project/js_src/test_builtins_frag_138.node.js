// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 138
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_138.node.js

function testBuiltins_frag_138() {
    try {

        function taylorSin(x) {
          return (n) => ((-1) ** n * x ** (2 * n + 1)) / factorial(2 * n + 1);
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_138] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_138();
}

module.exports = { testBuiltins_frag_138 };
