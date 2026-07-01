// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 48
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_48.node.js

function testBuiltins_frag_48() {
    try {

        /[ab]*/.exec("aba"); // ['aba']
        } catch (e) {
        console.error(`[testBuiltins_frag_48] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_48();
}

module.exports = { testBuiltins_frag_48 };
