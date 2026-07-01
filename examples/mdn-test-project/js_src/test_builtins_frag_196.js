// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 196
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_196.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_196() {

        try {
          throw new ReferenceError("Hello");
        } catch (e) {
          console.log(e instanceof ReferenceError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "ReferenceError"
          console.log(e.stack); // Stack of the error
        }
    }
