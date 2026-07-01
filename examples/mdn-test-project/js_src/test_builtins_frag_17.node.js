// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 17
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_17.node.js

function testBuiltins_frag_17() {
    try {

        "cat"[1]; // gives value "a"
        } catch (e) {
        console.error(`[testBuiltins_frag_17] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_17();
}

module.exports = { testBuiltins_frag_17 };
