// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 47
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_47.node.js

function testBuiltins_frag_47() {
    try {

        const re = /a{1, 3}/;
        re.test("aa"); // false
        re.test("a{1, 3}"); // true
        } catch (e) {
        console.error(`[testBuiltins_frag_47] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_47();
}

module.exports = { testBuiltins_frag_47 };
