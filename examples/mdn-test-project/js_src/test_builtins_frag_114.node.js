// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 114
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_114.node.js

function testBuiltins_frag_114() {
    try {

        parseFloat(3.14);
        parseFloat("3.14");
        parseFloat("  3.14  ");
        parseFloat("314e-2");
        parseFloat("0.0314E+2");
        parseFloat("3.14some non-digit characters");
        parseFloat({
          toString() {
            return "3.14";
          },
        });
        } catch (e) {
        console.error(`[testBuiltins_frag_114] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_114();
}

module.exports = { testBuiltins_frag_114 };
