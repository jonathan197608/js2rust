// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 6
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_6.node.js

function testBuiltins_frag_6() {
    try {

        const arr = [2, 4, NaN, 12];
        arr.indexOf(NaN); // -1
        arr.includes(NaN); // true

        // Methods accepting a properly defined predicate can always find NaN
        arr.findIndex((n) => Number.isNaN(n)); // 2
        } catch (e) {
        console.error(`[testBuiltins_frag_6] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_6();
}

module.exports = { testBuiltins_frag_6 };
