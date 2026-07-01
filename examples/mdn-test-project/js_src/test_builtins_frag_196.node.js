// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 196
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_196.node.js

function testBuiltins_frag_196() {
    try {

        try {
          throw new ReferenceError("Hello");
        } catch (e) {
          console.log(e instanceof ReferenceError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "ReferenceError"
          console.log(e.stack); // Stack of the error
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_196] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_196();
}

module.exports = { testBuiltins_frag_196 };
