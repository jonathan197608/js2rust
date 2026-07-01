// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 1
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_1.node.js

function testBuiltins_frag_1() {
    try {

        console.log(Infinity); /* Infinity */
        console.log(Infinity + 1); /* Infinity */
        console.log(10 ** 1000); /* Infinity */
        console.log(Math.log(0)); /* -Infinity */
        console.log(1 / Infinity); /* 0 */
        console.log(1 / 0); /* Infinity */
        } catch (e) {
        console.error(`[testBuiltins_frag_1] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_1();
}

module.exports = { testBuiltins_frag_1 };
