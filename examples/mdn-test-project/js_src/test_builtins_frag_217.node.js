// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 217
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_217.node.js

function testBuiltins_frag_217() {
    try {

        const mixed = [4n, 6, -12n, 10, 4, 0, 0n];
        // [4n, 6, -12n, 10, 4, 0, 0n]

        mixed.sort(); // default sorting behavior
        // [ -12n, 0, 0n, 10, 4n, 4, 6 ]

        mixed.sort((a, b) => a - b);
        // won't work since subtraction will not work with mixed types
        // TypeError: can't convert BigInt value to Number value

        // sort with an appropriate numeric comparator
        mixed.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
        // [ -12n, 0, 0n, 4n, 4, 6, 10 ]
        } catch (e) {
        console.error(`[testBuiltins_frag_217] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_217();
}

module.exports = { testBuiltins_frag_217 };
