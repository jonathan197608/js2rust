// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 197
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_197.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_197() {

        try {
          throw new SuppressedError(
            new Error("New error"),
            new Error("Original error"),
            "Hello",
          );
        } catch (e) {
          console.log(e instanceof SuppressedError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "SuppressedError"
          console.log(e.error); // Error: "New error"
          console.log(e.suppressed); // Error: "Original error"
        }
    }
