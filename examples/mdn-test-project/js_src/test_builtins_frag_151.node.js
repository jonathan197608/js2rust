// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 151
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_151.node.js

function testBuiltins_frag_151() {
    try {

        encodeURI("\uD800\uDFFF");
        // "%F0%90%8F%BF"
        } catch (e) {
        console.error(`[testBuiltins_frag_151] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_151();
}

module.exports = { testBuiltins_frag_151 };
