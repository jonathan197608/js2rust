// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 60
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_60.node.js

function testBuiltins_frag_60() {
    try {

        "foo".normalize("nfc"); // RangeError
        "foo".normalize(" NFC "); // RangeError
        } catch (e) {
        console.error(`[testBuiltins_frag_60] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_60();
}

module.exports = { testBuiltins_frag_60 };
