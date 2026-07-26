// Auto-generated edge-case test fragment (Zig transpile target)
// Category: edge, Fragment: 25
// Targeting recurring bug patterns from R17-R30 deep audits

export function testEdge_frag_25() {
    console.log((0.1 + 0.2).toFixed(2));
    console.log((1234.5678).toFixed(0));
    console.log((255).toString(16));
    console.log((255).toString(2));
    console.log(parseInt("0xff", 16));
    console.log(parseInt("101", 2));
    console.log(Number.MAX_SAFE_INTEGER);
    console.log(Number.isInteger(42.0));
    console.log(Number.isInteger(42.5));
    console.log(Number.isSafeInteger(9007199254740991));
    console.log(Number.isNaN(NaN));
    console.log(Number.isFinite(Infinity));
}
