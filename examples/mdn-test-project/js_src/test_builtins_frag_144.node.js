// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 144
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_144.node.js

function testBuiltins_frag_144() {
    try {

        const circularReference = { otherData: 123 };
        circularReference.myself = circularReference;
        } catch (e) {
        console.error(`[testBuiltins_frag_144] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_144();
}

module.exports = { testBuiltins_frag_144 };
