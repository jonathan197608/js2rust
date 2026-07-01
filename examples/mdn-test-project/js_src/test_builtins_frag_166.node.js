// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 166
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_166.node.js

function testBuiltins_frag_166() {
    try {

        parseInt(null, 36); // 1112745: The string "null" is 1112745 in base 36
        parseInt(undefined, 36); // 86464843759093: The string "undefined" is 86464843759093 in base 36
        } catch (e) {
        console.error(`[testBuiltins_frag_166] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_166();
}

module.exports = { testBuiltins_frag_166 };
