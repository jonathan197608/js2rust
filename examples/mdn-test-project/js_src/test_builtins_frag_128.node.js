// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 128
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_128.node.js

function testBuiltins_frag_128() {
    try {

        /b+/; // b is a character, it can be repeated
        /(\*hello\*)/; // Escape the asterisks to match them literally
        } catch (e) {
        console.error(`[testBuiltins_frag_128] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_128();
}

module.exports = { testBuiltins_frag_128 };
