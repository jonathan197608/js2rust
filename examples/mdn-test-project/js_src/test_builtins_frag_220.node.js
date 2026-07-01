// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 220
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_220.node.js

function testBuiltins_frag_220() {
    try {

        BigInt.prototype.toJSON = function () {
          return { $bigint: this.toString() };
        };
        } catch (e) {
        console.error(`[testBuiltins_frag_220] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_220();
}

module.exports = { testBuiltins_frag_220 };
