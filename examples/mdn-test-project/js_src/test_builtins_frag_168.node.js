// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 168
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_168.node.js

function testBuiltins_frag_168() {
    try {

        parseInt(4.7 * 1e22, 10); // Very large number becomes 4
        parseInt(0.00000000000434, 10); // Very small number becomes 4

        parseInt(0.0000001, 10); // 1
        parseInt(0.000000123, 10); // 1
        parseInt(1e-7, 10); // 1
        parseInt(1000000000000000000000, 10); // 1
        parseInt(123000000000000000000000, 10); // 1
        parseInt(1e21, 10); // 1
        } catch (e) {
        console.error(`[testBuiltins_frag_168] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_168();
}

module.exports = { testBuiltins_frag_168 };
