// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 8
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_8.node.js

function testBuiltins_frag_8() {
    try {

        NaN ** 0 === 1; // true
        } catch (e) {
        console.error(`[testBuiltins_frag_8] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_8();
}

module.exports = { testBuiltins_frag_8 };
