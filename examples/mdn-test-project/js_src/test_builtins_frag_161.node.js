// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 161
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_161.node.js

function testBuiltins_frag_161() {
    try {

        parseInt("-F", 16);
        parseInt("-0F", 16);
        parseInt("-0XF", 16);
        parseInt("-17", 8);
        parseInt("-15", 10);
        parseInt("-1111", 2);
        parseInt("-15e1", 10);
        parseInt("-12", 13);
        } catch (e) {
        console.error(`[testBuiltins_frag_161] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_161();
}

module.exports = { testBuiltins_frag_161 };
