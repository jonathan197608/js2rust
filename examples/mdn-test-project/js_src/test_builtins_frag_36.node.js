// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 36
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_36.node.js

function testBuiltins_frag_36() {
    try {

        function splitWords(str) {
          return str.split(/\s+/);
        }

        splitWords(`Look at the stars
        Look  how they\tshine for you`);
        // ['Look', 'at', 'the', 'stars', 'Look', 'how', 'they', 'shine', 'for', 'you']
        } catch (e) {
        console.error(`[testBuiltins_frag_36] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_36();
}

module.exports = { testBuiltins_frag_36 };
