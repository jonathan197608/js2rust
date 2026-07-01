// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 59
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_59.node.js

function testBuiltins_frag_59() {
    try {

        String.fromCodePoint("_"); // RangeError
        String.fromCodePoint(Infinity); // RangeError
        String.fromCodePoint(-1); // RangeError
        String.fromCodePoint(3.14); // RangeError
        String.fromCodePoint(3e-2); // RangeError
        String.fromCodePoint(NaN); // RangeError
        } catch (e) {
        console.error(`[testBuiltins_frag_59] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_59();
}

module.exports = { testBuiltins_frag_59 };
