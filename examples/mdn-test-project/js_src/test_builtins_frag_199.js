// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 199
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_199.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_199() {

        try {
          throw new SyntaxError("Hello");
        } catch (e) {
          console.log(e instanceof SyntaxError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "SyntaxError"
          console.log(e.stack); // Stack of the error
        }
    }
