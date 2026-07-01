// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 5
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_5.node.js

function testBuiltins_frag_5() {
    try {

        isNaN(1n); // TypeError: Conversion from 'BigInt' to 'number' is not allowed.
        Number.isNaN(1n); // false
        } catch (e) {
        console.error(`[testBuiltins_frag_5] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_5();
}

module.exports = { testBuiltins_frag_5 };
