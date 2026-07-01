// Auto-generated MDN test fragment (Zig transpile target)
// Category: builtins, Fragment: 192
// Source: test_builtins_part*.js
// Run with Node.js: node test_builtins_frag_192.node.js
// Transpile with js2rust: cargo build -p mdn-test-project

export function testBuiltins_frag_192() {

        Promise.any([Promise.reject(new Error("some error"))]).catch((e) => {
          console.log(e instanceof AggregateError); // true
          console.log(e.message); // "All Promises rejected"
          console.log(e.name); // "AggregateError"
          console.log(e.errors); // [ Error: "some error" ]
        });
    }
