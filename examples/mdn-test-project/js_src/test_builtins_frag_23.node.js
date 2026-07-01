// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 23
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_23.node.js

function testBuiltins_frag_23() {
    try {

        // You cannot access properties on null or undefined

        const nullVar = null;
        nullVar.toString(); // TypeError: Cannot read properties of null
        String(nullVar); // "null"

        const undefinedVar = undefined;
        undefinedVar.toString(); // TypeError: Cannot read properties of undefined
        String(undefinedVar); // "undefined"
        } catch (e) {
        console.error(`[testBuiltins_frag_23] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_23();
}

module.exports = { testBuiltins_frag_23 };
