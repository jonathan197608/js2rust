// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 214
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_214.node.js

function testBuiltins_frag_214() {
    try {

        const previousMaxSafe = BigInt(Number.MAX_SAFE_INTEGER); // 9007199254740991n
        const maxPlusOne = previousMaxSafe + 1n; // 9007199254740992n
        const theFuture = previousMaxSafe + 2n; // 9007199254740993n, this works now!
        const prod = previousMaxSafe * 2n; // 18014398509481982n
        const diff = prod - 10n; // 18014398509481972n
        const mod = prod % 10n; // 2n
        const bigN = 2n ** 54n; // 18014398509481984n
        bigN * -1n; // -18014398509481984n
        const expected = 4n / 2n; // 2n
        const truncated = 5n / 2n; // 2n, not 2.5n
        } catch (e) {
        console.error(`[testBuiltins_frag_214] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_214();
}

module.exports = { testBuiltins_frag_214 };
