// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 112
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_112.node.js

function testBuiltins_frag_112() {
    try {

        function circumference(r) {
          return parseFloat(r) * 2.0 * Math.PI;
        }

        console.log(circumference(4.567));

        console.log(circumference("4.567abcdefgh"));

        console.log(circumference("abcdefgh"));
        } catch (e) {
        console.error(`[testBuiltins_frag_112] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_112();
}

module.exports = { testBuiltins_frag_112 };
