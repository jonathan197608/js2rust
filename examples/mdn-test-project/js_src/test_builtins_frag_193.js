// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 193
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_193.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_193() {

        try {
          throw new AggregateError([new Error("some error")], "Hello");
        } catch (e) {
          console.log(e instanceof AggregateError); // true
          console.log(e.message); // "Hello"
          console.log(e.name); // "AggregateError"
          console.log(e.errors); // [ Error: "some error" ]
        }
    }
