// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 167
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_167.node.js

function testBuiltins_frag_167() {
    try {

        parseInt(15.99, 10); // 15
        parseInt(-15.1, 10); // -15
        } catch (e) {
        console.error(`[testBuiltins_frag_167] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_167();
}

module.exports = { testBuiltins_frag_167 };
