// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 178
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_178.node.js

function testBuiltins_frag_178() {
    try {

        function encodeRFC3986URI(str) {
          return encodeURI(str)
            .replace(/%5B/g, "[")
            .replace(/%5D/g, "]")
            .replace(
              /[!'()*]/g,
              (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`,
            );
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_178] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_178();
}

module.exports = { testBuiltins_frag_178 };
