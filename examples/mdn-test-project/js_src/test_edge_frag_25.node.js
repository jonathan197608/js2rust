// Auto-generated edge-case test fragment (Node.js reference runner)
// Category: edge, Fragment: 25

function testEdge_frag_25() {
    try {
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
    } catch (e) {
        console.error(`[testEdge_frag_25] error: `);
    }
}

if (require.main === module) {
    testEdge_frag_25();
}

module.exports = { testEdge_frag_25 };
