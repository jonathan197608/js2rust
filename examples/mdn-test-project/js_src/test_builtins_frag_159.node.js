// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 159
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_159.node.js

function testBuiltins_frag_159() {
    try {

        parseInt("0xF", 16);
        parseInt("F", 16);
        parseInt("17", 8);
        parseInt("015", 10);
        parseInt("15,123", 10);
        parseInt("FXX123", 16);
        parseInt("1111", 2);
        parseInt("15 * 3", 10);
        parseInt("15e2", 10);
        parseInt("15px", 10);
        parseInt("12", 13);
        } catch (e) {
        console.error(`[testBuiltins_frag_159] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_159();
}

module.exports = { testBuiltins_frag_159 };
