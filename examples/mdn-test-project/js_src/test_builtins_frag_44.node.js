// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 44
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_44.node.js

function testBuiltins_frag_44() {
    try {

        function isValidIdentifier(str) {
          return /^[$_\p{ID_Start}][$_\p{ID_Continue}]*$/u.test(str);
        }

        isValidIdentifier("foo"); // true
        isValidIdentifier("$1"); // true
        isValidIdentifier("1foo"); // false
        isValidIdentifier("  foo  "); // false
        } catch (e) {
        console.error(`[testBuiltins_frag_44] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_44();
}

module.exports = { testBuiltins_frag_44 };
