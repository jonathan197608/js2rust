// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 28
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_28.node.js

function testBuiltins_frag_28() {
    try {

        registry.register(theObject, "some value");
        } catch (e) {
        console.error(`[testBuiltins_frag_28] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_28();
}

module.exports = { testBuiltins_frag_28 };
