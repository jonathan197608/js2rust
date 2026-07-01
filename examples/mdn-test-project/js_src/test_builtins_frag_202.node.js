// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 202
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_202.node.js

function testBuiltins_frag_202() {
    try {

        try {
          decodeURIComponent("%");
        } catch (e) {
          console.log(e instanceof URIError); // true
          console.log(e.message); // "malformed URI sequence"
          console.log(e.name); // "URIError"
          console.log(e.stack); // Stack of the error
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_202] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_202();
}

module.exports = { testBuiltins_frag_202 };
