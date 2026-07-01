// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 206
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_206.node.js

function testBuiltins_frag_206() {
    try {

        new Int32Array([1.1, 1.9, -1.1, -1.9]); // Int32Array(4) [ 1, 1, -1, -1 ]

        new Int8Array([257, -257]); // Int8Array(2) [ 1, -1 ]
        // 257 = 0001 0000 0001
        //     =      0000 0001 (mod 2^8)
        //     = 1
        // -257 = 1110 1111 1111
        //      =      1111 1111 (mod 2^8)
        //      = -1 (as signed integer)

        new Uint8Array([257, -257]); // Uint8Array(2) [ 1, 255 ]
        // -257 = 1110 1111 1111
        //      =      1111 1111 (mod 2^8)
        //      = 255 (as unsigned integer)
        } catch (e) {
        console.error(`[testBuiltins_frag_206] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_206();
}

module.exports = { testBuiltins_frag_206 };
