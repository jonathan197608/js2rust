// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 7
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_7.node.js

function testBuiltins_frag_7() {
    try {

        const f2b = (x) => new Uint8Array(new Float64Array([x]).buffer);
        const b2f = (x) => new Float64Array(x.buffer)[0];
        // Get a byte representation of NaN
        const n = f2b(NaN);
        const m = f2b(NaN);
        // Change the sign bit, which doesn't matter for NaN
        n[7] += 2 ** 7;
        // n[0] += 2**7; for big endian processors
        const nan2 = b2f(n);
        console.log(nan2); // NaN
        console.log(Object.is(nan2, NaN)); // true
        console.log(f2b(NaN)); // Uint8Array(8) [0, 0, 0, 0, 0, 0, 248, 127]
        console.log(f2b(nan2)); // Uint8Array(8) [0, 0, 0, 0, 0, 0, 248, 255]
        // Change the first bit, which is the least significant bit of the mantissa and doesn't matter for NaN
        m[0] = 1;
        // m[7] = 1; for big endian processors
        const nan3 = b2f(m);
        console.log(nan3); // NaN
        console.log(Object.is(nan3, NaN)); // true
        console.log(f2b(NaN)); // Uint8Array(8) [0, 0, 0, 0, 0, 0, 248, 127]
        console.log(f2b(nan3)); // Uint8Array(8) [1, 0, 0, 0, 0, 0, 248, 127]
        } catch (e) {
        console.error(`[testBuiltins_frag_7] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_7();
}

module.exports = { testBuiltins_frag_7 };
