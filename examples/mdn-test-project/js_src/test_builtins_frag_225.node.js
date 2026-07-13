// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 225
// Source: ICU toLocaleLowerCase
// Run: node test_builtins_frag_225.node.js

function testBuiltins_frag_225() {
    try {

        // toLocaleLowerCase: basic ASCII lowercasing
        const r1 = "HELLO".toLocaleLowerCase(); // 'hello'
        const r2 = "HeLLo WoRLd".toLocaleLowerCase(); // 'hello world'

        console.log(r1, r2);
        } catch (e) {
        console.error(`[testBuiltins_frag_225] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_225();
}

module.exports = { testBuiltins_frag_225 };
