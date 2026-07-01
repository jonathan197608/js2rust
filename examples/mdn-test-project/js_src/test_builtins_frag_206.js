// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 206
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_206.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_206() {

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
    }
