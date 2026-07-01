// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 143
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_143.node.js

function testBuiltins_frag_143() {
    try {

        const obj = {};
        Object.preventExtensions(obj);
        Object.setPrototypeOf(obj, {});
        // TypeError: can't set prototype of this object
        } catch (e) {
        console.error(`[testBuiltins_frag_143] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_143();
}

module.exports = { testBuiltins_frag_143 };
