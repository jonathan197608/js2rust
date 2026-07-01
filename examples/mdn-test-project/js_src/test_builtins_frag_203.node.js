// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 203
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_203.node.js

function testBuiltins_frag_203() {
    try {

        try {
          throw new URIError("Hello");
        } catch (e) {
          console.log(e instanceof URIError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "URIError"
          console.log(e.stack); // Stack of the error
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_203] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_203();
}

module.exports = { testBuiltins_frag_203 };
