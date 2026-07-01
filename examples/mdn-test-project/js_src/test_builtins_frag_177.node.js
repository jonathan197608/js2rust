// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 177
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_177.node.js

function testBuiltins_frag_177() {
    try {

        // High-low pair OK
        encodeURI("\uD800\uDFFF"); // "%F0%90%8F%BF"

        // Lone high-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURI("\uD800");

        // Lone low-surrogate code unit throws "URIError: malformed URI sequence"
        encodeURI("\uDFFF");
        } catch (e) {
        console.error(`[testBuiltins_frag_177] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_177();
}

module.exports = { testBuiltins_frag_177 };
