// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 27
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_27.node.js

function testBuiltins_frag_27() {
    try {

        registry.register(target, "some value");
        } catch (e) {
        console.error(`[testBuiltins_frag_27] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_27();
}

module.exports = { testBuiltins_frag_27 };
