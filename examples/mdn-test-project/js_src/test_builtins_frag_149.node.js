// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 149
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_149.node.js

function testBuiltins_frag_149() {
    try {

        null.foo;
        // TypeError: null has no properties

        undefined.bar;
        // TypeError: undefined has no properties
        } catch (e) {
        console.error(`[testBuiltins_frag_149] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_149();
}

module.exports = { testBuiltins_frag_149 };
