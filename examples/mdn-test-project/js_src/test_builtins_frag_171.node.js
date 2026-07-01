// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 171
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_171.node.js

function testBuiltins_frag_171() {
    try {

        try {
          const a = decodeURI("%E0%A4%A");
        } catch (e) {
          console.error(e);
        }

        // URIError: malformed URI sequence
        } catch (e) {
        console.error(`[testBuiltins_frag_171] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_171();
}

module.exports = { testBuiltins_frag_171 };
