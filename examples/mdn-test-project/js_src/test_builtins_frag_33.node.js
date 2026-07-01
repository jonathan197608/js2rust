// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 33
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_33.node.js

function testBuiltins_frag_33() {
    try {

        const r1 = /\p{Lowercase_Letter}/iu;
        const r2 = /[^\P{Lowercase_Letter}]/iu;
        } catch (e) {
        console.error(`[testBuiltins_frag_33] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_33();
}

module.exports = { testBuiltins_frag_33 };
