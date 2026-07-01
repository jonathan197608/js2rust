// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 52
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_52.node.js

function testBuiltins_frag_52() {
    try {

        /(?=a)?b/.test("b"); // true; the lookahead is matched 0 time
        } catch (e) {
        console.error(`[testBuiltins_frag_52] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_52();
}

module.exports = { testBuiltins_frag_52 };
