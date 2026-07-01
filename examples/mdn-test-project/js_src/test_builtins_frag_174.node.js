// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 174
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_174.node.js

function testBuiltins_frag_174() {
    try {

        function decodeQueryParam(p) {
          return decodeURIComponent(p.replace(/\+/g, " "));
        }

        decodeQueryParam("search+query%20%28correct%29");
        // 'search query (correct)'
        } catch (e) {
        console.error(`[testBuiltins_frag_174] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_174();
}

module.exports = { testBuiltins_frag_174 };
