// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 221
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_221.node.js

function testBuiltins_frag_221() {
    try {

        console.log(JSON.stringify({ a: 1n }));
        // {"a":{"$bigint":"1"}}
        } catch (e) {
        console.error(`[testBuiltins_frag_221] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_221();
}

module.exports = { testBuiltins_frag_221 };
