// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 188
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_188.node.js

function testBuiltins_frag_188() {
    try {

        if (new Boolean(true)) {
          console.log("This log is printed.");
        }

        if (new Boolean(false)) {
          console.log("This log is ALSO printed.");
        }

        const myFalse = new Boolean(false); // myFalse is a Boolean object (not the primitive value false)
        const g = Boolean(myFalse); // g is true
        const myString = new String("Hello"); // myString is a String object
        const s = Boolean(myString); // s is true
        } catch (e) {
        console.error(`[testBuiltins_frag_188] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_188();
}

module.exports = { testBuiltins_frag_188 };
