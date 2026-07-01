// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 19
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_19.node.js

function testBuiltins_frag_19() {
    try {

        function areEqualCaseInsensitive(str1, str2) {
          return str1.toUpperCase() === str2.toUpperCase();
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_19] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_19();
}

module.exports = { testBuiltins_frag_19 };
