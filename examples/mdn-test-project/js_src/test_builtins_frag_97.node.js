// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 97
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_97.node.js

function testBuiltins_frag_97() {
    try {

        JSON.parse('{"foo": 1}');
        } catch (e) {
        console.error(`[testBuiltins_frag_97] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_97();
}

module.exports = { testBuiltins_frag_97 };
