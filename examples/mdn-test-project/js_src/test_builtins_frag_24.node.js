// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 24
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_24.node.js

function testBuiltins_frag_24() {
    try {

        const buffer = new ArrayBuffer(8);
        const view = new Int32Array(buffer);
        } catch (e) {
        console.error(`[testBuiltins_frag_24] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_24();
}

module.exports = { testBuiltins_frag_24 };
