// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 207
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_207.node.js

function testBuiltins_frag_207() {
    try {

        const biggestNum = Number.MAX_VALUE;
        const smallestNum = Number.MIN_VALUE;
        const infiniteNum = Number.POSITIVE_INFINITY;
        const negInfiniteNum = Number.NEGATIVE_INFINITY;
        const notANum = Number.NaN;
        } catch (e) {
        console.error(`[testBuiltins_frag_207] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_207();
}

module.exports = { testBuiltins_frag_207 };
