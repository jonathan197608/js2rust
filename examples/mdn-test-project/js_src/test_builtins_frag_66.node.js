// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 66
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_66.node.js

function testBuiltins_frag_66() {
    try {

        (77.1234).toExponential(4); // 7.7123e+1
        (77.1234).toExponential(2); // 7.71e+1

        (2.34).toFixed(1); // 2.3
        (2.35).toFixed(1); // 2.4 (note that it rounds up in this case)

        (5.123456).toPrecision(5); // 5.1235
        (5.123456).toPrecision(2); // 5.1
        (5.123456).toPrecision(1); // 5
        } catch (e) {
        console.error(`[testBuiltins_frag_66] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_66();
}

module.exports = { testBuiltins_frag_66 };
