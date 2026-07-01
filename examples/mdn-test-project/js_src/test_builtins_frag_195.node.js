// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 195
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_195.node.js

function testBuiltins_frag_195() {
    try {

        try {
          let a = undefinedVariable;
        } catch (e) {
          console.log(e instanceof ReferenceError); // true
          console.log(e.message); // "undefinedVariable is not defined"
          console.log(e.name); // "ReferenceError"
          console.log(e.stack); // Stack of the error
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_195] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_195();
}

module.exports = { testBuiltins_frag_195 };
