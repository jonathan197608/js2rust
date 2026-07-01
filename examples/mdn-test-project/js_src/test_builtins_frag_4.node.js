// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 4
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_4.node.js

function testBuiltins_frag_4() {
    try {

        isNaN("hello world"); // true
        Number.isNaN("hello world"); // false
        } catch (e) {
        console.error(`[testBuiltins_frag_4] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_4();
}

module.exports = { testBuiltins_frag_4 };
