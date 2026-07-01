// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 199
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_199.node.js

function testBuiltins_frag_199() {
    try {

        try {
          throw new SyntaxError("Hello");
        } catch (e) {
          console.log(e instanceof SyntaxError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "SyntaxError"
          console.log(e.stack); // Stack of the error
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_199] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_199();
}

module.exports = { testBuiltins_frag_199 };
