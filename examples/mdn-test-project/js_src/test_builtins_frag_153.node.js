// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 153
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_153.node.js

function testBuiltins_frag_153() {
    try {

        parseFloat("1.7976931348623159e+308"); // Infinity
        parseFloat("-1.7976931348623159e+308"); // -Infinity
        } catch (e) {
        console.error(`[testBuiltins_frag_153] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_153();
}

module.exports = { testBuiltins_frag_153 };
