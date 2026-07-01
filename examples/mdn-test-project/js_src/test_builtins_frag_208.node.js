// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 208
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_208.node.js

function testBuiltins_frag_208() {
    try {

        const biggestInt = Number.MAX_SAFE_INTEGER; // (2**53 - 1) => 9007199254740991
        const smallestInt = Number.MIN_SAFE_INTEGER; // -(2**53 - 1) => -9007199254740991
        } catch (e) {
        console.error(`[testBuiltins_frag_208] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_208();
}

module.exports = { testBuiltins_frag_208 };
