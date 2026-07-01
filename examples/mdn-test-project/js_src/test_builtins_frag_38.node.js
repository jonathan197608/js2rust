// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 38
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_38.node.js

function testBuiltins_frag_38() {
    try {

        const pattern = /a\nb/;
        const string = `a
        b`;
        console.log(pattern.test(string)); // true
        } catch (e) {
        console.error(`[testBuiltins_frag_38] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_38();
}

module.exports = { testBuiltins_frag_38 };
