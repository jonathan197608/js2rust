// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 72
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_72.node.js

function testBuiltins_frag_72() {
    try {

        "abc".repeat(0); // ''
        "abc".repeat(1); // 'abc'
        "abc".repeat(2); // 'abcabc'
        "abc".repeat(3.5); // 'abcabcabc' (count will be converted to integer)
        } catch (e) {
        console.error(`[testBuiltins_frag_72] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_72();
}

module.exports = { testBuiltins_frag_72 };
