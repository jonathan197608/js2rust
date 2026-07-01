// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 9
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_9.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_9() {

        function div(x) {
          if (isFinite(1000 / x)) {
            return "Number is NOT Infinity.";
          }
          return "Number is Infinity!";
        }

        console.log(div(0));

        console.log(div(1));
    }
