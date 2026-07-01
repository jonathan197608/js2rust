// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 26
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_26.node.js

function testBuiltins_frag_26() {
    try {

        const buffer = new ArrayBuffer(16);
        const view = new DataView(buffer, 0);

        view.setInt16(1, 42);
        view.getInt16(1); // 42
        } catch (e) {
        console.error(`[testBuiltins_frag_26] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_26();
}

module.exports = { testBuiltins_frag_26 };
