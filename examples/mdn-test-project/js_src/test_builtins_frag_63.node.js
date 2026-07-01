// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 63
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_63.node.js

function testBuiltins_frag_63() {
    try {

        invalid.toString(); // "Invalid Date"
        invalid.getDate(); // NaN
        } catch (e) {
        console.error(`[testBuiltins_frag_63] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_63();
}

module.exports = { testBuiltins_frag_63 };
