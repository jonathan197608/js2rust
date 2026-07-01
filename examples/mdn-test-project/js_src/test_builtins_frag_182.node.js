// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 182
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_182.node.js

function testBuiltins_frag_182() {
    try {

        // High-low pair OK
        encodeURIComponent("\uD800\uDFFF"); // "%F0%90%8F%BF"

        // Lone high-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURIComponent("\uD800");

        // Lone high-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURIComponent("\uDFFF");
        } catch (e) {
        console.error(`[testBuiltins_frag_182] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_182();
}

module.exports = { testBuiltins_frag_182 };
