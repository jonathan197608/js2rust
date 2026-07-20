// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 228
// Source: BigInt.prototype.toString(radix)
// Run with Node.js: node test_builtins_frag_228.node.js
// Transpile with js2rust: cargo build -p mdn-test-project
// R8-P1-4: BigInt.prototype.toString(radix) — radix argument was previously
// silently dropped; now correctly forwarded to the runtime.

export function testBuiltins_frag_228() {
    console.log((255n).toString(16));     // "ff"
    console.log((255n).toString(2));      // "11111111"
    console.log((255n).toString());       // "255"
    console.log((0n).toString(2));        // "0"
    const big = 1000000n;
    console.log(big.toString(36));        // "lfls"
}
