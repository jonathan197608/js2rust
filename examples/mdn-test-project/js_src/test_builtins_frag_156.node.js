// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 156
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_156.node.js

function testBuiltins_frag_156() {
    try {
        const huge = BigInt("900719925474099267");
        console.log(huge);
    } catch (e) {
        console.error(`[testBuiltins_frag_156] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_156();
}

module.exports = { testBuiltins_frag_156 };
