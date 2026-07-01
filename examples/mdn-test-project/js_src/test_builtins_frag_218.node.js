// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 218
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_218.node.js

function testBuiltins_frag_218() {
    try {

        Object(0n) === 0n; // false
        Object(0n) === Object(0n); // false

        const o = Object(0n);
        o === o; // true
        } catch (e) {
        console.error(`[testBuiltins_frag_218] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_218();
}

module.exports = { testBuiltins_frag_218 };
