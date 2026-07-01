// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 53
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_53.node.js

function testBuiltins_frag_53() {
    try {

        function countParagraphs(str) {
          return str.match(/(?:\r?\n){2,}/g).length + 1;
        }

        countParagraphs(`
        Paragraph 1

        Paragraph 2
        Containing some line breaks, but still the same paragraph

        Another paragraph
        `); // 3
        } catch (e) {
        console.error(`[testBuiltins_frag_53] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_53();
}

module.exports = { testBuiltins_frag_53 };
