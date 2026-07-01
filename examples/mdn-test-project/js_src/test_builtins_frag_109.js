// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 109
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_109.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_109() {

        function milliseconds(x) {
          if (isNaN(x)) {
            return "Not a Number!";
          }
          return x * 1000;
        }

        console.log(milliseconds("100F"));

        console.log(milliseconds("0.0314E+2"));
    }
