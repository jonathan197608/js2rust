// Auto-generated MDN test fragment (Node.js reference runner)
// Category: builtins, Fragment: 185
// Source: test_builtins_part*.js
// Run: node test_builtins_frag_185.node.js

function testBuiltins_frag_185() {
    try {

        // Create a global property with `var`
        var x = 10;

        function createFunction1() {
          const x = 20;
          return new Function("return x;"); // this `x` refers to global `x`
        }

        function createFunction2() {
          const x = 20;
          function f() {
            return x; // this `x` refers to the local `x` above
          }
          return f;
        }

        const f1 = createFunction1();
        console.log(f1()); // 10
        const f2 = createFunction2();
        console.log(f2()); // 20
        } catch (e) {
        console.error(`[testBuiltins_frag_185] error: ${e.message}`);
    }
}

// Self-test when run directly
if (require.main === module) {
    testBuiltins_frag_185();
}

module.exports = { testBuiltins_frag_185 };
