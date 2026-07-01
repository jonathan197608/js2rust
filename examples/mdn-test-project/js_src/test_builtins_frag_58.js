// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 58
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_58.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_58() {

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
    }
