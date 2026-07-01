// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 210
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_210.node.js

function testBuiltins_frag_210() {
    try {

        Number("123"); // 123
        Number("123") === 123; // true
        Number("12.3"); // 12.3
        Number("12.00"); // 12
        Number("123e-1"); // 12.3
        Number(""); // 0
        Number(null); // 0
        Number("0x11"); // 17
        Number("0b11"); // 3
        Number("0o11"); // 9
        Number("foo"); // NaN
        Number("100a"); // NaN
        Number("-Infinity"); // -Infinity
        } catch (e) {
        console.error(`[testBuiltins_frag_210] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_210();
}

module.exports = { testBuiltins_frag_210 };
