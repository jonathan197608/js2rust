// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 61
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_61.node.js

function testBuiltins_frag_61() {
    try {

        "foo".normalize("NFC"); // 'foo'
        } catch (e) {
        console.error(`[testBuiltins_frag_61] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_61();
}

module.exports = { testBuiltins_frag_61 };
