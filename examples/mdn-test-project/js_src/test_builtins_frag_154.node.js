// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 154
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_154.node.js

function testBuiltins_frag_154() {
    try {

        parseFloat("Infinity"); // Infinity
        parseFloat("-Infinity"); // -Infinity
        } catch (e) {
        console.error(`[testBuiltins_frag_154] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_154();
}

module.exports = { testBuiltins_frag_154 };
