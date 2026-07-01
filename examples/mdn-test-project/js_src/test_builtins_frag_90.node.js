// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 90
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_90.node.js

function testBuiltins_frag_90() {
    try {

        /[1-9]/; // Swap the range
        /[_\-=]/; // Escape the hyphen so it matches the literal character
        } catch (e) {
        console.error(`[testBuiltins_frag_90] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_90();
}

module.exports = { testBuiltins_frag_90 };
