// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 214
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_214.node.js

function testBuiltins_frag_214() {
    try {
        const previousMaxSafe = BigInt(9007199254740991);
        const maxPlusOne = previousMaxSafe + 1n;
        const theFuture = previousMaxSafe + 2n;
        const prod = previousMaxSafe * 2n;
        const diff = prod - 10n;
        const mod = prod % 10n;
        const bigN = 2n ** 54n;
        const neg = bigN * -1n;
        const expected = 4n / 2n;
        const truncated = 5n / 2n;
        console.log(maxPlusOne);
        console.log(theFuture);
        console.log(prod);
        console.log(diff);
        console.log(mod);
        console.log(bigN);
        console.log(neg);
        console.log(expected);
        console.log(truncated);
    } catch (e) {
        console.error(`[testBuiltins_frag_214] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_214();
}

module.exports = { testBuiltins_frag_214 };
