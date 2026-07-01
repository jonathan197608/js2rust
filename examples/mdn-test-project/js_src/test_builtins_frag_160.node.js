// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 160
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_160.node.js

function testBuiltins_frag_160() {
    try {

        parseInt("Hello", 8); // Not a number at all
        parseInt("546", 2); // Digits other than 0 or 1 are invalid for binary radix
        } catch (e) {
        console.error(`[testBuiltins_frag_160] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_160();
}

module.exports = { testBuiltins_frag_160 };
