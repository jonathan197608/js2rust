// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 34
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_34.node.js

function testBuiltins_frag_34() {
    try {

        function isHexadecimal(str) {
          return /^[0-9A-F]+$/i.test(str);
        }

        isHexadecimal("2F3"); // true
        isHexadecimal("beef"); // true
        isHexadecimal("undefined"); // false
        } catch (e) {
        console.error(`[testBuiltins_frag_34] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_34();
}

module.exports = { testBuiltins_frag_34 };
