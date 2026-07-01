// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 141
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_141.node.js

function testBuiltins_frag_141() {
    try {

        Object.defineProperty({}, "key", { value: "foo", writable: false });
        } catch (e) {
        console.error(`[testBuiltins_frag_141] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_141();
}

module.exports = { testBuiltins_frag_141 };
