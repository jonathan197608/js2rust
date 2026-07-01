// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 65
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_65.node.js

function testBuiltins_frag_65() {
    try {

        (77.1234).toExponential(-1); // RangeError
        (77.1234).toExponential(101); // RangeError

        (2.34).toFixed(-100); // RangeError
        (2.34).toFixed(1001); // RangeError

        (1234.5).toPrecision(-1); // RangeError
        (1234.5).toPrecision(101); // RangeError
        } catch (e) {
        console.error(`[testBuiltins_frag_65] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_65();
}

module.exports = { testBuiltins_frag_65 };
