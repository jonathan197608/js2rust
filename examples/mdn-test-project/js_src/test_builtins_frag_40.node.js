// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 40
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_40.node.js

function testBuiltins_frag_40() {
    try {

        /(?:(a)|(ab))(?:(c)|(bc))/.exec("abc"); // ['abc', 'a', undefined, undefined, 'bc']
        // Not ['abc', undefined, 'ab', 'c', undefined]
        } catch (e) {
        console.error(`[testBuiltins_frag_40] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_40();
}

module.exports = { testBuiltins_frag_40 };
