// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 42
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_42.node.js

function testBuiltins_frag_42() {
    try {

        function removeTrailingSlash(url) {
          return url.replace(/\/$/, "");
        }

        removeTrailingSlash("https://example.com/"); // "https://example.com"
        removeTrailingSlash("https://example.com/docs/"); // "https://example.com/docs"
        } catch (e) {
        console.error(`[testBuiltins_frag_42] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_42();
}

module.exports = { testBuiltins_frag_42 };
