// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 87
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_87.node.js

function testBuiltins_frag_87() {
    try {

        // If you want to match NULL followed by a digit, use a character class
        /[\0]0/u;
        // If you want to match a character by its character value, use \x
        /\x01/u;
        } catch (e) {
        console.error(`[testBuiltins_frag_87] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_87();
}

module.exports = { testBuiltins_frag_87 };
