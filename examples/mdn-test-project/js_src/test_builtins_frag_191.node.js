// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 191
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_191.node.js

function testBuiltins_frag_191() {
    try {

        const btrue = Boolean(true);
        const btrueString = Boolean("true");
        const bfalseString = Boolean("false");
        const bSuLin = Boolean("Su Lin");
        const bArrayProto = Boolean([]);
        const bObjProto = Boolean({});
        } catch (e) {
        console.error(`[testBuiltins_frag_191] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_191();
}

module.exports = { testBuiltins_frag_191 };
