// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 96
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_96.node.js

function testBuiltins_frag_96() {
    try {

        JSON.parse("[1, 2, 3, 4]");
        JSON.parse('{"foo": 1}');
        } catch (e) {
        console.error(`[testBuiltins_frag_96] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_96();
}

module.exports = { testBuiltins_frag_96 };
