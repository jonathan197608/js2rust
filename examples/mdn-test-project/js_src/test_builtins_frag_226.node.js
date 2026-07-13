// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 226
// Source: ICU normalize
// Run: node test_builtins_frag_226.node.js

function testBuiltins_frag_226() {
    try {

        // normalize: returns copy of input string
        const r1 = "hello".normalize(); // 'hello' (default NFC)
        const r2 = "cafe".normalize("NFD"); // 'cafe' (simplified impl returns input)

        console.log(r1, r2);
        } catch (e) {
        console.error(`[testBuiltins_frag_226] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_226();
}

module.exports = { testBuiltins_frag_226 };
