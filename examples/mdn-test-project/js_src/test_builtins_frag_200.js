// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 200
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_200.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_200() {

        try {
          null.f();
        } catch (e) {
          console.log(e instanceof TypeError); // true
          console.log(e.message); // "null has no properties"
          console.log(e.name); // "TypeError"
          console.log(e.stack); // Stack of the error
        }
    }
