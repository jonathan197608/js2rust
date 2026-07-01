// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 195
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_195.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_195() {

        try {
          let a = undefinedVariable;
        } catch (e) {
          console.log(e instanceof ReferenceError); // true
          console.log(e.message); // "undefinedVariable is not defined"
          console.log(e.name); // "ReferenceError"
          console.log(e.stack); // Stack of the error
        }
    }
