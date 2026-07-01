// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 112
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_112.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_112() {

        function circumference(r) {
          return parseFloat(r) * 2.0 * Math.PI;
        }

        console.log(circumference(4.567));

        console.log(circumference("4.567abcdefgh"));

        console.log(circumference("abcdefgh"));
    }
