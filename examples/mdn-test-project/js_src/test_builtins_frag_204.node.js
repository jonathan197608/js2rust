// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 204
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_204.node.js

function testBuiltins_frag_204() {
    try {

        255; // two-hundred and fifty-five
        255.0; // same number
        255 === 255.0; // true
        255 === 0xff; // true (hexadecimal notation)
        255 === 0b11111111; // true (binary notation)
        255 === 0.255e3; // true (decimal exponential notation)
        } catch (e) {
        console.error(`[testBuiltins_frag_204] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_204();
}

module.exports = { testBuiltins_frag_204 };
