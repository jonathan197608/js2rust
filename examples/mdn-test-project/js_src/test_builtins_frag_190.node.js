// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 190
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_190.node.js

function testBuiltins_frag_190() {
    try {

        const bNoParam = Boolean();
        const bZero = Boolean(0);
        const bNull = Boolean(null);
        const bEmptyString = Boolean("");
        const bfalse = Boolean(false);
        } catch (e) {
        console.error(`[testBuiltins_frag_190] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_190();
}

module.exports = { testBuiltins_frag_190 };
