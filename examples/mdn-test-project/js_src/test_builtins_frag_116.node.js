// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 116
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_116.node.js

function testBuiltins_frag_116() {
    try {

        parseFloat("NaN"); // NaN
        } catch (e) {
        console.error(`[testBuiltins_frag_116] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_116();
}

module.exports = { testBuiltins_frag_116 };
