// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 64
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_64.node.js

function testBuiltins_frag_64() {
    try {

        new Date("05 October 2011 14:48 UTC").toISOString(); // "2011-10-05T14:48:00.000Z"
        new Date(1317826080).toISOString(); // "2011-10-05T14:48:00.000Z"
        } catch (e) {
        console.error(`[testBuiltins_frag_64] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_64();
}

module.exports = { testBuiltins_frag_64 };
