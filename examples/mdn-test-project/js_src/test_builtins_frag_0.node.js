// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 0
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_0.node.js

function testBuiltins_frag_0() {
    try {

        const maxNumber = 10 ** 1000; // Max positive number

        if (maxNumber === Infinity) {
          console.log("Let's call it Infinity!");
        }

        console.log(1 / maxNumber);
        } catch (e) {
        console.error(`[testBuiltins_frag_0] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_0();
}

module.exports = { testBuiltins_frag_0 };
