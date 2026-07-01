// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 18
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_18.node.js

function testBuiltins_frag_18() {
    try {

        const a = "a";
        const b = "b";
        if (a < b) {
          // true
          console.log(`${a} is less than ${b}`);
        } else if (a > b) {
          console.log(`${a} is greater than ${b}`);
        } else {
          console.log(`${a} and ${b} are equal.`);
        }
        } catch (e) {
        console.error(`[testBuiltins_frag_18] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_18();
}

module.exports = { testBuiltins_frag_18 };
