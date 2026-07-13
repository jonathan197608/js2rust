// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 227
// Source: Math.sign
// Run: node test_builtins_frag_227.node.js

function testBuiltins_frag_227() {
    try {
        console.log(Math.sign(3));
        console.log(Math.sign(-3));
        console.log(Math.sign(0));
        console.log(Math.sign(42));
        console.log(Math.sign(-42));
    } catch (e) {
        console.error(`[testBuiltins_frag_227] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_227();
}

module.exports = { testBuiltins_frag_227 };
