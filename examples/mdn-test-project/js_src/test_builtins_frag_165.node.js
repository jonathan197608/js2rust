// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 165
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_165.node.js

function testBuiltins_frag_165() {
    try {

        parseInt("123_456"); // 123
        } catch (e) {
        console.error(`[testBuiltins_frag_165] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_165();
}

module.exports = { testBuiltins_frag_165 };
