// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 69
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_69.node.js

function testBuiltins_frag_69() {
    try {

        "abc".repeat(Infinity); // RangeError
        "a".repeat(2 ** 30); // RangeError
        } catch (e) {
        console.error(`[testBuiltins_frag_69] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_69();
}

module.exports = { testBuiltins_frag_69 };
