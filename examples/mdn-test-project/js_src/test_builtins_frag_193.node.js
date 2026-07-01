// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 193
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_193.node.js

function testBuiltins_frag_193() {
    try {

        try {
          throw new AggregateError([new Error("some error")], "Hello");
        } catch (e) {
          console.log(e instanceof AggregateError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "AggregateError"
          console.log(e.errors); // [ Error: "some error" ]
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_193] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_193();
}

module.exports = { testBuiltins_frag_193 };
