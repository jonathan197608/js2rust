// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 20
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_20.node.js

function testBuiltins_frag_20() {
    try {

        const strPrim = "foo"; // A literal is a string primitive
        const strPrim2 = String(1); // Coerced into the string primitive "1"
        const strPrim3 = String(true); // Coerced into the string primitive "true"
        const strObj = new String(strPrim); // String with new returns a string wrapper object.

        console.log(typeof strPrim); // "string"
        console.log(typeof strPrim2); // "string"
        console.log(typeof strPrim3); // "string"
        console.log(typeof strObj); // "object"
        } catch (e) {
        console.error(`[testBuiltins_frag_20] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_20();
}

module.exports = { testBuiltins_frag_20 };
