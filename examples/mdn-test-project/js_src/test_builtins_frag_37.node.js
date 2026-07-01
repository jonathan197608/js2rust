// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 37
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_37.node.js

function testBuiltins_frag_37() {
    try {

        /[\c0]/.test("\x10"); // true
        /[\c_]/.test("\x1f"); // true
        /[\c*]/.test("\\"); // true
        /\c/.test("\\c"); // true
        /\c0/.test("\\c0"); // true (the \c0 syntax is only supported in character classes)
        } catch (e) {
        console.error(`[testBuiltins_frag_37] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_37();
}

module.exports = { testBuiltins_frag_37 };
