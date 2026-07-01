// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 216
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_216.node.js

function testBuiltins_frag_216() {
    try {

        1n < 2; // true
        2n > 1; // true
        2 > 2; // false
        2n > 2; // false
        2n >= 2; // true
        } catch (e) {
        console.error(`[testBuiltins_frag_216] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_216();
}

module.exports = { testBuiltins_frag_216 };
