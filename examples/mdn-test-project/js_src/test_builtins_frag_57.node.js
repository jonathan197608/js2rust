// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 57
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_57.node.js

function testBuiltins_frag_57() {
    try {

        function hasThanks(str) {
          return /\b(thanks|thank you)\b/i.test(str);
        }

        hasThanks("Thanks! You helped me a lot."); // true
        hasThanks("Just want to say thank you for all your work."); // true
        hasThanks("Thanksgiving is around the corner."); // false
        } catch (e) {
        console.error(`[testBuiltins_frag_57] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_57();
}

module.exports = { testBuiltins_frag_57 };
