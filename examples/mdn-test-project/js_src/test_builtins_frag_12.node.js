// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 12
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_12.node.js

function testBuiltins_frag_12() {
    try {

        50 * Math.tan(degToRad(60));
        } catch (e) {
        console.error(`[testBuiltins_frag_12] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_12();
}

module.exports = { testBuiltins_frag_12 };
