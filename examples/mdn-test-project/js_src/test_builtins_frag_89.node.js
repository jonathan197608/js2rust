// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 89
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_89.node.js

function testBuiltins_frag_89() {
    try {

        /\p{Script=Latin}/u; // "Script=Latin" is a valid Unicode property
        /\p{Letter}/u; // "Letter" is valid value for General_Category
        /\p{RGI_Emoji_Flag_Sequence}/v; // Property of strings can only be used in "v" mode
        } catch (e) {
        console.error(`[testBuiltins_frag_89] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_89();
}

module.exports = { testBuiltins_frag_89 };
