// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 194
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_194.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_194() {

        try {
          throw new EvalError("Hello");
        } catch (e) {
          console.log(e instanceof EvalError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "EvalError"
          console.log(e.stack); // Stack of the error
        }
    }
