// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 56
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_56.node.js

function testBuiltins_frag_56() {
    try {

        /\ba/.exec("abc");
        /c\b/.exec("abc");

        /\B /.exec(" abc");
        / \B/.exec("abc ");
        } catch (e) {
        console.error(`[testBuiltins_frag_56] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_56();
}

module.exports = { testBuiltins_frag_56 };
