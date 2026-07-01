// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 50
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_50.node.js

function testBuiltins_frag_50() {
    try {

        /a*?$/.exec("aaa"); // ['aaa']; the match already succeeds at the first character, so the regex never attempts to start matching at the second character
        } catch (e) {
        console.error(`[testBuiltins_frag_50] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_50();
}

module.exports = { testBuiltins_frag_50 };
