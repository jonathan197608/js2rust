// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 51
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_51.node.js

function testBuiltins_frag_51() {
    try {

        /[ab]+[abc]c/.exec("abbc"); // ['abbc']
        } catch (e) {
        console.error(`[testBuiltins_frag_51] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_51();
}

module.exports = { testBuiltins_frag_51 };
