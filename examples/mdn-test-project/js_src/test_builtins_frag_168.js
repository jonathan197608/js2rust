// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 168
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_168.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_168() {

        parseInt(4.7 * 1e22, 10); // Very large number becomes 4
        parseInt(0.00000000000434, 10); // Very small number becomes 4

        parseInt(0.0000001, 10); // 1
        parseInt(0.000000123, 10); // 1
        parseInt(1e-7, 10); // 1
        parseInt(1000000000000000000000, 10); // 1
        parseInt(123000000000000000000000, 10); // 1
        parseInt(1e21, 10); // 1
    }
