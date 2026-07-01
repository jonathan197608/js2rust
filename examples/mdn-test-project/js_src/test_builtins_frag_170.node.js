// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 170
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_170.node.js

function testBuiltins_frag_170() {
    try {

        decodeURI(
          "https://developer.mozilla.org/docs/JavaScript%3A%20a_scripting_language",
        );
        // "https://developer.mozilla.org/docs/JavaScript%3A a_scripting_language"

        decodeURIComponent(
          "https://developer.mozilla.org/docs/JavaScript%3A%20a_scripting_language",
        );
        // "https://developer.mozilla.org/docs/JavaScript: a_scripting_language"
        } catch (e) {
        console.error(`[testBuiltins_frag_170] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_170();
}

module.exports = { testBuiltins_frag_170 };
