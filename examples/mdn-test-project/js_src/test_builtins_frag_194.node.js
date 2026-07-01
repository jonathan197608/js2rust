// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 194
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_194.node.js

function testBuiltins_frag_194() {
    try {

        try {
          throw new EvalError("Hello");
        } catch (e) {
          console.log(e instanceof EvalError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "EvalError"
          console.log(e.stack); // Stack of the error
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_194] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_194();
}

module.exports = { testBuiltins_frag_194 };
