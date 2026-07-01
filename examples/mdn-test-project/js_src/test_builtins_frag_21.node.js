// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 21
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_21.node.js

function testBuiltins_frag_21() {
    try {

        const s1 = "2 + 2"; // creates a string primitive
        const s2 = new String("2 + 2"); // creates a String object
        console.log(eval(s1)); // returns the number 4
        console.log(eval(s2)); // returns the string "2 + 2"
        } catch (e) {
        console.error(`[testBuiltins_frag_21] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_21();
}

module.exports = { testBuiltins_frag_21 };
