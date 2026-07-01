// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 162
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_162.node.js

function testBuiltins_frag_162() {
    try {

        parseInt("0e0", 16);
        } catch (e) {
        console.error(`[testBuiltins_frag_162] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_162();
}

module.exports = { testBuiltins_frag_162 };
