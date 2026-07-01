// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 132
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_132.node.js

function testBuiltins_frag_132() {
    try {

        function replacer(match, ...args) {
          const offset = args.at(-2);
          const string = args.at(-1);
        }

        function doSomething(arg1, arg2, ...otherArgs) {}
        } catch (e) {
        console.error(`[testBuiltins_frag_132] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_132();
}

module.exports = { testBuiltins_frag_132 };
