// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 134
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_134.node.js

function testBuiltins_frag_134() {
    try {

        // All { and } need to be escaped
        /\{\{MDN_Macro\}\}/u;
        // The ] needs to be escaped
        /\[sic\]/u;
        } catch (e) {
        console.error(`[testBuiltins_frag_134] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_134();
}

module.exports = { testBuiltins_frag_134 };
