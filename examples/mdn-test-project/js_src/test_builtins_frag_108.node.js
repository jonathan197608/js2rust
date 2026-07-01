// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 108
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_108.node.js

function testBuiltins_frag_108() {
    try {

        isFinite(Infinity); // false
        isFinite(NaN); // false
        isFinite(-Infinity); // false

        isFinite(0); // true
        isFinite(2e64); // true
        isFinite(910); // true

        // Would've been false with the more robust Number.isFinite():
        isFinite(null); // true
        isFinite("0"); // true
        } catch (e) {
        console.error(`[testBuiltins_frag_108] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_108();
}

module.exports = { testBuiltins_frag_108 };
