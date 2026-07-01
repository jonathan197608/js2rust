// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 146
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_146.node.js

function testBuiltins_frag_146() {
    try {

        "abc".matchAll(/./); // TypeError
        "abc".replaceAll(/./, "f"); // TypeError
        } catch (e) {
        console.error(`[testBuiltins_frag_146] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_146();
}

module.exports = { testBuiltins_frag_146 };
