// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 224
// Source: ICU toLocaleUpperCase
// Run: node test_builtins_frag_224.node.js

function testBuiltins_frag_224() {
    try {

        // toLocaleUpperCase: basic ASCII uppercasing
        const r1 = "hello".toLocaleUpperCase(); // 'HELLO'
        const r2 = "HeLLo WoRLd".toLocaleUpperCase(); // 'HELLO WORLD'

        console.log(r1, r2);
        } catch (e) {
        console.error(`[testBuiltins_frag_224] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_224();
}

module.exports = { testBuiltins_frag_224 };
