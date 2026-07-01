// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 16
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_16.node.js

function testBuiltins_frag_16() {
    try {

        "cat".charAt(1); // gives value "a"
        } catch (e) {
        console.error(`[testBuiltins_frag_16] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_16();
}

module.exports = { testBuiltins_frag_16 };
