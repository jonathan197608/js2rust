// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 81
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_81.node.js

function testBuiltins_frag_81() {
    try {

        const arr = ["a", "b", "c"];

        for (let i = 2; i < arr.length; i++) {
          console.log(arr[i]);
        }

        // "c"
        } catch (e) {
        console.error(`[testBuiltins_frag_81] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_81();
}

module.exports = { testBuiltins_frag_81 };
