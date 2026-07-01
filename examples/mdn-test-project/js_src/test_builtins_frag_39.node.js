// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 39
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_39.node.js

function testBuiltins_frag_39() {
    try {

        /a|ab/.exec("abc"); // ['a']
        } catch (e) {
        console.error(`[testBuiltins_frag_39] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_39();
}

module.exports = { testBuiltins_frag_39 };
