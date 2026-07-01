// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 58
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_58.node.js

function testBuiltins_frag_58() {
    try {

        var a = 2;
        try {
          throw new Error();
        } catch (a) {
          var a = 1; // This 1 is assigned to the caught `a`, not the outer `a`.
        }
        console.log(a); // 2

        try {
          throw new Error();
          // Note: identifier changed to `err` to avoid conflict with
          // the inner declaration of `a`.
        } catch (err) {
          var a = 1; // This 1 is assigned to the upper-scope `a`.
        }
        console.log(a); // 1
        } catch (e) {
        console.error(`[testBuiltins_frag_58] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_58();
}

module.exports = { testBuiltins_frag_58 };
