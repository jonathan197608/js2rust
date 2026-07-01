// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 3
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_3.node.js

function testBuiltins_frag_3() {
    try {

        NaN === NaN; // false
        Number.NaN === NaN; // false
        isNaN(NaN); // true
        isNaN(Number.NaN); // true
        Number.isNaN(NaN); // true

        function valueIsNaN(v) {
          return v !== v;
        }
        valueIsNaN(1); // false
        valueIsNaN(NaN); // true
        valueIsNaN(Number.NaN); // true
        } catch (e) {
        console.error(`[testBuiltins_frag_3] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_3();
}

module.exports = { testBuiltins_frag_3 };
