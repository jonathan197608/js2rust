// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 127
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_127.node.js

function testBuiltins_frag_127() {
    try {

        // Matches two characters that are not an emoji flag sequence
        /(?!\p{RGI_Emoji_Flag_Sequence})../v;
        } catch (e) {
        console.error(`[testBuiltins_frag_127] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_127();
}

module.exports = { testBuiltins_frag_127 };
