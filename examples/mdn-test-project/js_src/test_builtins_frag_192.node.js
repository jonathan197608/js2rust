// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 192
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_192.node.js

function testBuiltins_frag_192() {
    try {

        Promise.any([Promise.reject(new Error("some error"))]).catch((e) => {
          console.log(e instanceof AggregateError); // true
          console.log(e.message); // "All Promises rejected"
          console.log(e.name); // "AggregateError"
          console.log(e.errors); // [ Error: "some error" ]
        });
        } catch (e) {
        console.error(`[testBuiltins_frag_192] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_192();
}

module.exports = { testBuiltins_frag_192 };
