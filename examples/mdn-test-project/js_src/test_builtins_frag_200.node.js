// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 200
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_200.node.js

function testBuiltins_frag_200() {
    try {

        try {
          null.f();
        } catch (e) {
          console.log(e instanceof TypeError); // true
          console.log(e.message); // "null has no properties"
          console.log(e.name); // "TypeError"
          console.log(e.stack); // Stack of the error
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_200] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_200();
}

module.exports = { testBuiltins_frag_200 };
