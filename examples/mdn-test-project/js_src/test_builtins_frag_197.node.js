// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 197
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_197.node.js

function testBuiltins_frag_197() {
    try {

        try {
          throw new SuppressedError(
            new Error("New error"),
            new Error("Original error"),
            "Hello",
          );
        } catch (e) {
          console.log(e instanceof SuppressedError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "SuppressedError"
          console.log(e.error); // Error: "New error"
          console.log(e.suppressed); // Error: "Original error"
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_197] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_197();
}

module.exports = { testBuiltins_frag_197 };
