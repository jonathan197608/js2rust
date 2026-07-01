// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 212
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_212.node.js

function testBuiltins_frag_212() {
    try {

        typeof 1n === "bigint"; // true
        typeof BigInt("1") === "bigint"; // true
        } catch (e) {
        console.error(`[testBuiltins_frag_212] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_212();
}

module.exports = { testBuiltins_frag_212 };
