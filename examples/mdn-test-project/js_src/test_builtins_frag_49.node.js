// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 49
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_49.node.js

function testBuiltins_frag_49() {
    try {

        /a*/.exec("aaa"); // ['aaa']; the entire input is consumed
        /a*?/.exec("aaa"); // ['']; it's possible to consume no characters and still match successfully
        /^a*?$/.exec("aaa"); // ['aaa']; it's not possible to consume fewer characters and still match successfully
        } catch (e) {
        console.error(`[testBuiltins_frag_49] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_49();
}

module.exports = { testBuiltins_frag_49 };
