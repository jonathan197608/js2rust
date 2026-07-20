// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 228
// Source: BigInt.prototype.toString(radix)
// Run: node test_builtins_frag_228.node.js

function testBuiltins_frag_228() {
    try {
        console.log((255n).toString(16));     // "ff"
        console.log((255n).toString(2));      // "11111111"
        console.log((255n).toString());       // "255"
        console.log((0n).toString(2));        // "0"
        const big = 1000000n;
        console.log(big.toString(36));        // "lfls"
    } catch (e) {
        console.error(`[testBuiltins_frag_228] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_228();
}

module.exports = { testBuiltins_frag_228 };
