// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 152
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_152.node.js

function testBuiltins_frag_152() {
    try {

        decodeURIComponent("%E0%A4%A");
        // "URIError: malformed URI sequence"
        } catch (e) {
        console.error(`[testBuiltins_frag_152] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_152();
}

module.exports = { testBuiltins_frag_152 };
