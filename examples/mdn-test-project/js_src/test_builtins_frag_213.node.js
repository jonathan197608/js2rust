// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 213
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_213.node.js

function testBuiltins_frag_213() {
    try {

        typeof Object(1n) === "object"; // true
        } catch (e) {
        console.error(`[testBuiltins_frag_213] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_213();
}

module.exports = { testBuiltins_frag_213 };
