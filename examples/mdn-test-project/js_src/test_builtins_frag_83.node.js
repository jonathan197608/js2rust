// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 83
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_83.node.js

function testBuiltins_frag_83() {
    try {

        // Wrap the number in parentheses
        alert(typeof (1).toString());

        // Add an extra dot for the number literal
        alert(typeof 2..toString());

        // Use square brackets
        alert(typeof 3["toString"]());
        } catch (e) {
        console.error(`[testBuiltins_frag_83] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_83();
}

module.exports = { testBuiltins_frag_83 };
