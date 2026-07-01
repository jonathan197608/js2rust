// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 2
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_2.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_2() {

        function sanitize(x) {
          if (isNaN(x)) {
            return NaN;
          }
          return x;
        }

        console.log(sanitize("1"));

        console.log(sanitize("NotANumber"));
    }
