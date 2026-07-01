// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 145
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_145.node.js

function testBuiltins_frag_145() {
    try {

        JSON.stringify(circularReference);
        // TypeError: cyclic object value
        } catch (e) {
        console.error(`[testBuiltins_frag_145] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_145();
}

module.exports = { testBuiltins_frag_145 };
