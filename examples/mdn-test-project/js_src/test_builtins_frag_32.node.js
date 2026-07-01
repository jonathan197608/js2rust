// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 32
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_32.node.js

function testBuiltins_frag_32() {
    try {

        /[\s-9]/.test("-"); // true
        } catch (e) {
        console.error(`[testBuiltins_frag_32] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_32();
}

module.exports = { testBuiltins_frag_32 };
