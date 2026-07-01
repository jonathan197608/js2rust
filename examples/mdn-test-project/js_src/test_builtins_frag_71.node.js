// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 71
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_71.node.js

function testBuiltins_frag_71() {
    try {

        "abc".repeat(-1); // RangeError
        } catch (e) {
        console.error(`[testBuiltins_frag_71] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_71();
}

module.exports = { testBuiltins_frag_71 };
